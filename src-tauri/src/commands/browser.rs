use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewBuilder};

/// Global state to track the browser webview label
pub struct BrowserState {
    pub webview_label: Mutex<Option<String>>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            webview_label: Mutex::new(None),
        }
    }
}

const BROWSER_LABEL: &str = "browser-webview";

/// JavaScript initialization script injected into every page load in the browser webview.
/// This is set via `initialization_script()` on WebviewBuilder so it runs automatically
/// on EVERY navigation — no need to manually re-inject.
///
/// Detection strategy (broad, catches most HLS/DASH players):
/// 1. Intercept fetch() — catches hls.js, dash.js, and most modern players
/// 2. Intercept XMLHttpRequest.open() — catches older players, jwplayer, etc.
/// 3. PerformanceObserver on "resource" — catches ALL network requests including
///    those made by native <video> elements and MSE (this is the key one for sites
///    like hanime that use custom players)
/// 4. MutationObserver — watches for <video>/<source> elements added to DOM
/// 5. Periodic polling of <video>.src — catches dynamically set sources
/// 6. Intercept MediaSource.addSourceBuffer — catches MSE-based players
///
/// Since __TAURI_INTERNALS__ may not be available in child webviews, we use
/// window.ipc.postMessage() which IS available in Tauri 2 child webviews.
fn stream_detection_script() -> String {
    r#"
    (function() {
        if (window.__cybersnatcher_injected) return;
        window.__cybersnatcher_injected = true;

        const seen = new Set();

        function getPageTitle() {
            try { var og = document.querySelector('meta[property="og:title"]');
                  if (og && og.getAttribute('content')) return og.getAttribute('content').trim(); } catch(e) {}
            try { var tw = document.querySelector('meta[name="twitter:title"]');
                  if (tw && tw.getAttribute('content')) return tw.getAttribute('content').trim(); } catch(e) {}
            try { var h1 = document.querySelector('h1');
                  if (h1 && h1.textContent && h1.textContent.trim()) return h1.textContent.trim(); } catch(e) {}
            return document.title || '';
        }

        function sendStreamEvent(url, streamType) {
            var title = getPageTitle();
            var payload = JSON.stringify({
                manifest_url: url,
                stream_type: streamType,
                page_url: window.location.href,
                page_title: title
            });

            // Try all available IPC methods
            try {
                if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                    window.__TAURI_INTERNALS__.invoke('on_stream_detected', {
                        manifestUrl: url,
                        streamType: streamType,
                        pageUrl: window.location.href,
                        pageTitle: title
                    }).catch(function() {});
                    return;
                }
            } catch(e) {}

            try {
                if (window.ipc && window.ipc.postMessage) {
                    window.ipc.postMessage('stream_detected:' + payload);
                    return;
                }
            } catch(e) {}

            try {
                if (window.__TAURI_IPC__) {
                    window.__TAURI_IPC__('stream_detected:' + payload);
                    return;
                }
            } catch(e) {}
        }

        function notifyStream(url, streamType) {
            if (!url || seen.has(url)) return;
            seen.add(url);
            sendStreamEvent(url, streamType);
        }

        // Save original fetch before hooking — used by checkUrl for m3u8 classification
        var origFetch = window.fetch;

        function checkUrl(url) {
            if (!url || typeof url !== 'string') return;
            if (seen.has(url)) return;
            try {
                var lower = url.toLowerCase();
                var isM3u8 = lower.includes('.m3u8');
                var isMpd = lower.includes('.mpd');

                if (isM3u8 && origFetch) {
                    // Mark as seen immediately to prevent duplicate processing
                    seen.add(url);
                    // Fetch playlist content to classify master vs media
                    origFetch(url).then(function(r) { return r.text(); }).then(function(text) {
                        if (text.indexOf('#EXT-X-STREAM-INF') !== -1) {
                            // Master playlist — extract variant URLs and suppress them
                            var lines = text.split('\n');
                            for (var i = 0; i < lines.length; i++) {
                                var line = lines[i].trim();
                                if (line && !line.startsWith('#')) {
                                    try {
                                        var variantUrl = new URL(line, url).href;
                                        seen.add(variantUrl);
                                    } catch(e2) {}
                                }
                            }
                            sendStreamEvent(url, 'hls');
                        } else if (text.indexOf('#EXTINF') !== -1 || text.indexOf('#EXTM3U') !== -1) {
                            // Media playlist (standalone, not a variant of a known master)
                            sendStreamEvent(url, 'hls');
                        }
                    }).catch(function() {
                        // Fetch failed — notify anyway, backend will validate
                        sendStreamEvent(url, 'hls');
                    });
                } else if (isM3u8) {
                    // No fetch available, fall back to direct notify
                    notifyStream(url, 'hls');
                } else if (isMpd) {
                    notifyStream(url, 'dash');
                }
            } catch(e) {}
        }

        function checkContentType(url, ct) {
            if (!ct || !url) return;
            ct = ct.toLowerCase();
            if (ct.includes('mpegurl') || ct.includes('x-mpegurl')) {
                checkUrl(url);
            } else if (ct.includes('dash+xml')) {
                notifyStream(url, 'dash');
            }
        }

        // 1. Intercept fetch()
        if (window.fetch) {
            window.fetch = function() {
                var input = arguments[0];
                var url = typeof input === 'string' ? input : (input && input.url ? input.url : '');
                checkUrl(url);
                return origFetch.apply(this, arguments).then(function(response) {
                    try {
                        checkUrl(response.url);
                        checkContentType(response.url, response.headers.get('content-type'));
                    } catch(e) {}
                    return response;
                });
            };
        }

        // 2. Intercept XMLHttpRequest
        var origXHROpen = XMLHttpRequest.prototype.open;
        var origXHRSend = XMLHttpRequest.prototype.send;
        XMLHttpRequest.prototype.open = function(method, url) {
            this.__cs_url = url;
            checkUrl(String(url));
            return origXHROpen.apply(this, arguments);
        };
        XMLHttpRequest.prototype.send = function() {
            var self = this;
            this.addEventListener('load', function() {
                try {
                    var ct = self.getResponseHeader('content-type');
                    checkContentType(self.__cs_url || self.responseURL, ct);
                    checkUrl(self.responseURL);
                } catch(e) {}
            });
            return origXHRSend.apply(this, arguments);
        };

        // 3. PerformanceObserver — catches ALL network requests
        // This is critical for catching requests made by native video elements,
        // MSE players, and any other method that bypasses fetch/XHR interception
        try {
            var perfObserver = new PerformanceObserver(function(list) {
                var entries = list.getEntries();
                for (var i = 0; i < entries.length; i++) {
                    var entry = entries[i];
                    if (entry.name) {
                        checkUrl(entry.name);
                    }
                }
            });
            perfObserver.observe({ type: 'resource', buffered: true });
        } catch(e) {}

        // 4. MutationObserver — watch for video/source elements
        try {
            var domObserver = new MutationObserver(function(mutations) {
                for (var i = 0; i < mutations.length; i++) {
                    var m = mutations[i];
                    for (var j = 0; j < m.addedNodes.length; j++) {
                        var node = m.addedNodes[j];
                        if (node.nodeType !== 1) continue;
                        if (node.tagName === 'VIDEO' || node.tagName === 'SOURCE' || node.tagName === 'IFRAME') {
                            checkUrl(node.src || node.getAttribute('src') || '');
                        }
                        if (node.querySelectorAll) {
                            var elems = node.querySelectorAll('video, source, video source');
                            for (var k = 0; k < elems.length; k++) {
                                checkUrl(elems[k].src || elems[k].getAttribute('src') || '');
                            }
                        }
                    }
                    // Also check attribute changes on video/source
                    if (m.type === 'attributes' && m.target && m.target.tagName) {
                        if (m.target.tagName === 'VIDEO' || m.target.tagName === 'SOURCE') {
                            checkUrl(m.target.src || m.target.getAttribute('src') || '');
                        }
                    }
                }
            });
            domObserver.observe(document.documentElement, {
                childList: true,
                subtree: true,
                attributes: true,
                attributeFilter: ['src']
            });
        } catch(e) {}

        // 5. Periodic polling — catches dynamically set video sources
        setInterval(function() {
            try {
                var videos = document.querySelectorAll('video');
                for (var i = 0; i < videos.length; i++) {
                    var v = videos[i];
                    if (v.src) checkUrl(v.src);
                    if (v.currentSrc) checkUrl(v.currentSrc);
                    // Check source children
                    var sources = v.querySelectorAll('source');
                    for (var j = 0; j < sources.length; j++) {
                        if (sources[j].src) checkUrl(sources[j].src);
                    }
                }
            } catch(e) {}
        }, 3000);

        // 6. Intercept MediaSource.addSourceBuffer — detect MSE MIME types
        try {
            if (window.MediaSource && window.MediaSource.prototype.addSourceBuffer) {
                var origAddSB = window.MediaSource.prototype.addSourceBuffer;
                window.MediaSource.prototype.addSourceBuffer = function(mime) {
                    // MSE is being used; the actual URLs come through fetch/XHR/perf observer
                    return origAddSB.apply(this, arguments);
                };
            }
        } catch(e) {}

        // 7. Check existing video elements immediately
        try {
            document.querySelectorAll('video, source').forEach(function(el) {
                checkUrl(el.src || el.getAttribute('src') || '');
                if (el.currentSrc) checkUrl(el.currentSrc);
            });
        } catch(e) {}

        // Also scan after a delay for lazy-loaded players
        setTimeout(function() {
            try {
                document.querySelectorAll('video, source').forEach(function(el) {
                    checkUrl(el.src || el.getAttribute('src') || '');
                    if (el.currentSrc) checkUrl(el.currentSrc);
                });
            } catch(e) {}
        }, 2000);

        setTimeout(function() {
            try {
                document.querySelectorAll('video, source').forEach(function(el) {
                    checkUrl(el.src || el.getAttribute('src') || '');
                    if (el.currentSrc) checkUrl(el.currentSrc);
                });
            } catch(e) {}
        }, 5000);
    })();
    "#.to_string()
}

#[tauri::command]
pub async fn create_browser_webview(app: AppHandle, url: String) -> Result<(), String> {
    let state = app.state::<BrowserState>();

    // Destroy existing webview if any
    {
        let mut label = state.webview_label.lock().unwrap();
        if let Some(ref existing) = *label {
            if let Some(wv) = app.get_webview(existing) {
                let _ = wv.close();
            }
        }
        *label = None;
    }

    let main_window = app.get_window("main").ok_or("Main window not found")?;

    let script = stream_detection_script();

    // Create the browser webview with on_navigation callback and initialization script
    let app_clone = app.clone();
    let app_clone2 = app.clone();
    let builder = WebviewBuilder::new(BROWSER_LABEL, WebviewUrl::External(
        url.parse().map_err(|e| format!("Invalid URL: {}", e))?
    ))
    .initialization_script(&script)
    .on_navigation(move |nav_url: &url::Url| {
        let _ = app_clone.emit("browser-navigated", serde_json::json!({ "url": nav_url.as_str() }));
        true
    })
    .on_page_load(move |_wv, payload| {
        match payload.event() {
            tauri::webview::PageLoadEvent::Started => {
                let _ = app_clone2.emit("browser-loading", serde_json::json!({ "loading": true }));
            }
            tauri::webview::PageLoadEvent::Finished => {
                let _ = app_clone2.emit("browser-loading", serde_json::json!({ "loading": false }));
                let _ = app_clone2.emit("browser-navigated", serde_json::json!({ "url": payload.url().as_str() }));
            }
            _ => {}
        }
    });

    let _webview = main_window
        .add_child(builder, tauri::LogicalPosition::new(0.0, -9999.0), tauri::LogicalSize::new(1.0, 1.0))
        .map_err(|e| format!("Failed to create webview: {}", e))?;

    {
        let mut label = state.webview_label.lock().unwrap();
        *label = Some(BROWSER_LABEL.to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn destroy_browser_webview(app: AppHandle) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let mut label = state.webview_label.lock().unwrap();
    if let Some(ref existing) = *label {
        if let Some(wv) = app.get_webview(existing) {
            wv.close().map_err(|e| e.to_string())?;
        }
    }
    *label = None;
    Ok(())
}

#[tauri::command]
pub async fn navigate_browser(app: AppHandle, url: String) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let label = state.webview_label.lock().unwrap();
    if let Some(ref label) = *label {
        if let Some(wv) = app.get_webview(label) {
            let parsed: url::Url = url.parse().map_err(|e| format!("Invalid URL: {}", e))?;
            wv.navigate(parsed).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn browser_go_back(app: AppHandle) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let label = state.webview_label.lock().unwrap();
    if let Some(ref label) = *label {
        if let Some(wv) = app.get_webview(label) {
            let _ = wv.eval("window.history.back()");
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn browser_go_forward(app: AppHandle) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let label = state.webview_label.lock().unwrap();
    if let Some(ref label) = *label {
        if let Some(wv) = app.get_webview(label) {
            let _ = wv.eval("window.history.forward()");
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn browser_reload(app: AppHandle) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let label = state.webview_label.lock().unwrap();
    if let Some(ref label) = *label {
        if let Some(wv) = app.get_webview(label) {
            let _ = wv.eval("window.location.reload()");
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn resize_browser_webview(
    app: AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let label = state.webview_label.lock().unwrap();
    if let Some(ref label) = *label {
        if let Some(wv) = app.get_webview(label) {
            let _ = wv.set_position(tauri::LogicalPosition::new(x, y));
            let _ = wv.set_size(tauri::LogicalSize::new(width, height));
        }
    }
    Ok(())
}

/// Called by the injected JS when a stream URL is detected in the browser
#[tauri::command]
pub async fn on_stream_detected(
    app: AppHandle,
    manifest_url: String,
    stream_type: String,
    page_url: String,
    page_title: String,
) -> Result<(), String> {
    log::info!(
        "Stream detected: type={}, url={}, page={}",
        stream_type, manifest_url, page_url
    );

    let _ = app.emit(
        "stream-detected-raw",
        serde_json::json!({
            "manifest_url": manifest_url,
            "stream_type": stream_type,
            "page_url": page_url,
            "page_title": page_title,
        }),
    );

    Ok(())
}
