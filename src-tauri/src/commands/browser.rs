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

/// JavaScript to inject into the browser webview for stream detection.
/// Intercepts fetch() and XMLHttpRequest to detect .m3u8 and .mpd URLs.
fn stream_detection_script() -> String {
    r#"
    (function() {
        if (window.__cybersnatcher_injected) return;
        window.__cybersnatcher_injected = true;

        const seen = new Set();

        function checkUrl(url) {
            if (!url || typeof url !== 'string') return;
            try {
                const lower = url.toLowerCase();
                let streamType = null;

                if (lower.includes('.m3u8') || lower.includes('application/vnd.apple.mpegurl')) {
                    streamType = 'hls';
                } else if (lower.includes('.mpd') || lower.includes('application/dash+xml')) {
                    streamType = 'dash';
                }

                if (streamType && !seen.has(url)) {
                    seen.add(url);
                    // Send to Tauri backend via postMessage
                    if (window.__TAURI_INTERNALS__) {
                        window.__TAURI_INTERNALS__.invoke('on_stream_detected', {
                            manifestUrl: url,
                            streamType: streamType,
                            pageUrl: window.location.href,
                            pageTitle: document.title || ''
                        }).catch(function() {});
                    }
                }
            } catch(e) {}
        }

        // Intercept fetch
        const origFetch = window.fetch;
        window.fetch = function() {
            const url = arguments[0];
            if (typeof url === 'string') checkUrl(url);
            else if (url && url.url) checkUrl(url.url);
            return origFetch.apply(this, arguments);
        };

        // Intercept XMLHttpRequest
        const origOpen = XMLHttpRequest.prototype.open;
        XMLHttpRequest.prototype.open = function() {
            if (arguments[1]) checkUrl(String(arguments[1]));
            return origOpen.apply(this, arguments);
        };

        // Intercept Response headers for content-type detection
        const origFetchBound = origFetch.bind(window);
        window.fetch = function() {
            const url = arguments[0];
            if (typeof url === 'string') checkUrl(url);
            else if (url && url.url) checkUrl(url.url);
            return origFetchBound.apply(null, arguments).then(function(response) {
                const ct = response.headers.get('content-type') || '';
                if (ct.includes('mpegurl') || ct.includes('dash+xml')) {
                    checkUrl(response.url);
                }
                return response;
            });
        };

        // Monitor video/source elements
        const observer = new MutationObserver(function(mutations) {
            for (const m of mutations) {
                for (const node of m.addedNodes) {
                    if (node.tagName === 'VIDEO' || node.tagName === 'SOURCE') {
                        const src = node.src || node.getAttribute('src') || '';
                        checkUrl(src);
                    }
                    if (node.querySelectorAll) {
                        node.querySelectorAll('video, source').forEach(function(el) {
                            const src = el.src || el.getAttribute('src') || '';
                            checkUrl(src);
                        });
                    }
                }
            }
        });
        observer.observe(document.documentElement, { childList: true, subtree: true });

        // Check existing video elements
        document.querySelectorAll('video, source').forEach(function(el) {
            const src = el.src || el.getAttribute('src') || '';
            checkUrl(src);
        });
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

    // Create the browser webview
    let webview = WebviewBuilder::new(BROWSER_LABEL, WebviewUrl::External(
        url.parse().map_err(|e| format!("Invalid URL: {}", e))?
    ))
    .auto_resize();

    let webview = main_window
        .add_child(webview, tauri::LogicalPosition::new(0.0, 80.0), tauri::LogicalSize::new(800.0, 600.0))
        .map_err(|e| format!("Failed to create webview: {}", e))?;

    // Inject stream detection script on page load
    let script = stream_detection_script();
    let _ = webview.eval(&script);

    // Listen for navigation events
    let app_clone = app.clone();
    webview.on_navigation(move |nav_url| {
        let _ = app_clone.emit("browser-navigated", serde_json::json!({ "url": nav_url.as_str() }));
        true
    });

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
            // Re-inject detection script after navigation
            let script = stream_detection_script();
            let _ = wv.eval(&script);
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
