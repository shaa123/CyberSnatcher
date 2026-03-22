// commands/browser.rs — Built-in browser with video detection
// Creates a child webview INSIDE the main window, injects video detection
// hooks to find HLS/DASH/direct video URLs.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};

// ── State ────────────────────────────────────────────────────────────────────

pub struct BrowserState {
    pub server_port: Mutex<Option<u16>>,
    pub detected: Mutex<Vec<DetectedVideo>>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            server_port: Mutex::new(None),
            detected: Mutex::new(vec![]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedVideo {
    pub url: String,
    pub video_type: String,
    pub label: String,
    pub page_url: String,
    pub page_title: String,
    /// File size in bytes (from HEAD check)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    /// Cookies from the browser session (for auth-gated streams)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<String>,
}

// ── Local detection server ───────────────────────────────────────────────────

fn ensure_server(app: &AppHandle) -> u16 {
    let state = app.state::<BrowserState>();
    let mut port_lock = state.server_port.lock().unwrap();

    if let Some(port) = *port_lock {
        return port;
    }

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind detection server");
    let port = listener.local_addr().unwrap().port();
    *port_lock = Some(port);

    let app_handle = app.clone();

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            handle_request(stream, &app_handle);
        }
    });

    port
}

fn handle_request(mut stream: std::net::TcpStream, app: &AppHandle) {
    // ── Read headers until \r\n\r\n ──────────────────────────────────────────
    let mut header_buf: Vec<u8> = Vec::with_capacity(8 * 1024);
    let mut tmp = [0u8; 4096];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) | Err(_) => return,
            Ok(n) => header_buf.extend_from_slice(&tmp[..n]),
        }
        if header_buf.len() > 64 * 1024 { return; } // headers too large
        if find_header_end(&header_buf).is_some() { break; }
    }

    let header_end = match find_header_end(&header_buf) {
        Some(pos) => pos,
        None => return,
    };

    let header_str = String::from_utf8_lossy(&header_buf[..header_end]).to_string();
    let first_line = header_str.lines().next().unwrap_or("");

    // ── Route: GET /report?data=... (URL detection) ──────────────────────────
    if first_line.starts_with("GET") && first_line.contains("/report?data=") {
        if let Some(encoded) = first_line
            .split("/report?data=")
            .nth(1)
            .and_then(|s| s.split(' ').next())
        {
            let decoded = percent_decode(encoded);
            if let Ok(video) = serde_json::from_str::<DetectedVideo>(&decoded) {
                let state = app.state::<BrowserState>();
                let mut detected = state.detected.lock().unwrap();
                if !detected.iter().any(|v| v.url == video.url) {
                    if video.video_type == "hls" || video.video_type == "dash" {
                        detected.push(video.clone());
                        let _ = app.emit("browser-video-detected", &video);
                    } else {
                        let video_clone = video.clone();
                        let app_clone = app.clone();
                        std::thread::spawn(move || {
                            head_check_and_add(video_clone, &app_clone);
                        });
                    }
                }
            }
        }
        send_ok(&mut stream);
        return;
    }

    // Fallback
    send_ok(&mut stream);
}

fn send_ok(stream: &mut std::net::TcpStream) {
    let response = "HTTP/1.1 200 OK\r\n\
        Access-Control-Allow-Origin: *\r\n\
        Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
        Access-Control-Allow-Headers: Content-Type\r\n\
        Content-Length: 2\r\n\
        Content-Type: text/plain\r\n\
        Connection: close\r\n\r\nOK";
    let _ = stream.write_all(response.as_bytes());
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    for i in 0..data.len().saturating_sub(3) {
        if data[i] == b'\r' && data[i + 1] == b'\n' && data[i + 2] == b'\r' && data[i + 3] == b'\n' {
            return Some(i + 4);
        }
    }
    None
}

fn percent_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(
                &String::from_utf8_lossy(&bytes[i + 1..i + 3]),
                16,
            ) {
                result.push(val);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            result.push(b' ');
            i += 1;
            continue;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

// ── HEAD request size check ──────────────────────────────────────────────────
// Sends HTTP HEAD to get Content-Length, filters out videos under 1MB,
// then adds to detected list sorted by size (biggest first).

const MIN_VIDEO_SIZE: u64 = 1_048_576; // 1MB — anything smaller is junk

fn head_check_and_add(mut video: DetectedVideo, app: &AppHandle) {
    // Try to get file size via HEAD request
    let size = get_content_length(&video.url);

    if let Some(s) = size {
        if s < MIN_VIDEO_SIZE {
            return; // Too small — thumbnail, ad, preview, skip it
        }
        video.file_size = Some(s);
    }
    // If HEAD fails (no Content-Length), still add it but with no size

    let state = app.state::<BrowserState>();
    let mut detected = state.detected.lock().unwrap();

    // Double-check it wasn't added while we were doing the HEAD request
    if detected.iter().any(|v| v.url == video.url) { return; }

    detected.push(video.clone());

    // Re-sort: biggest first, unknowns at the end
    detected.sort_by(|a, b| {
        let sa = a.file_size.unwrap_or(0);
        let sb = b.file_size.unwrap_or(0);
        sb.cmp(&sa)
    });

    let _ = app.emit("browser-video-detected", &video);
}

fn get_content_length(url: &str) -> Option<u64> {
    // Parse the URL to get host, port, path
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    let port = parsed.port_or_known_default()?;
    let path = if parsed.query().is_some() {
        format!("{}?{}", parsed.path(), parsed.query().unwrap())
    } else {
        parsed.path().to_string()
    };

    let is_https = parsed.scheme() == "https";

    // Build HEAD request
    let request = format!(
        "HEAD {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: Mozilla/5.0\r\nConnection: close\r\n\r\n",
        path, host
    );

    let response = if is_https {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .ok()?;
        let resp = client.head(url).send().ok()?;
        let cl = resp.headers().get("content-length")?;
        return cl.to_str().ok()?.parse::<u64>().ok();
    } else {
        // Plain HTTP — use TcpStream
        let addr = format!("{}:{}", host, port);
        let mut stream = std::net::TcpStream::connect_timeout(
            &addr.parse().ok()?,
            std::time::Duration::from_secs(5),
        ).ok()?;
        stream.write_all(request.as_bytes()).ok()?;
        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).ok()?;
        String::from_utf8_lossy(&buf[..n]).to_string()
    };

    // Parse Content-Length from response headers
    for line in response.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("content-length:") {
            let val = line.split(':').nth(1)?.trim();
            return val.parse::<u64>().ok();
        }
    }
    None
}

// ── Injection script ─────────────────────────────────────────────────────────
// Injected into every page the browser webview loads.
// Detects video URLs via DOM scanning, fetch/XHR hooks, MutationObserver.

fn make_inject_script(port: u16) -> String {
    format!(
        r#"
(function() {{
  'use strict';
  if (window.__cs_injected) return;
  window.__cs_injected = true;

  // ── VIDEO DETECTION ────────────────────────────────────────────────────
  const PORT = {port};
  const BASE = 'http://127.0.0.1:' + PORT;
  const reported = new Set();

  // ── Smart filtering patterns ──
  const AD_PAT = /ads?[_\-.]|track(ing)?|beacon|pixel|analytics|prebid|imasdk|doubleclick|googlesyndication|moatads|scorecardresearch/i;
  const THUMB_PAT = /thumb|preview|poster|sprite|placeholder|gif\.mp4|_default\.|rollover|\/key[s]?[\/\?]|\/key[0-9a-f]|_thumb|_small|_mini|sample|trailer_|teaser/i;
  const SKIP_EXT = /\.(php|html?|aspx?|jsp|json|js|css|png|jpe?g|gif|svg|woff2?|ico|ttf|eot|xml|m4s|m4f|cmfv|cmfa)(\?|#|$)/i;
  const SKIP_PATH = /\/(embed|watch|view_video|player|login|signup|register|checkout)\b/i;
  const TINY_PATH = /[_\-](120|160|180|240|320)p?[_\-.]|[_\-]small|[_\-]thumb|[_\-]preview|_low\b/i;
  const VID = /\.(mp4|webm|mkv|avi|mov|flv|wmv|m4v|m3u8|mpd|ts)([\/\?#]|$)/i;

  function isJunk(url) {{
    if (AD_PAT.test(url) || THUMB_PAT.test(url) || SKIP_EXT.test(url) || TINY_PATH.test(url)) return true;
    if (SKIP_PATH.test(url) && !VID.test(url)) return true;
    return false;
  }}

  // ── Helper: report a detected video URL ──
  function report(url, vtype, label) {{
    if (!url || reported.has(url)) return;
    if (url.startsWith('blob:') || url.startsWith('data:')) return;
    if (isJunk(url)) return;
    reported.add(url);
    try {{
      const data = JSON.stringify({{
        url: url,
        video_type: vtype,
        label: label,
        page_url: location.href,
        page_title: document.title,
        cookies: document.cookie || null
      }});
      new Image().src = BASE + '/report?data=' + encodeURIComponent(data);
    }} catch(e) {{}}
  }}

  function classify(url) {{
    if (/\.m3u8/i.test(url)) return ['hls', 'HLS'];
    if (/\.mpd/i.test(url)) return ['dash', 'DASH'];
    const m = url.match(/\.(mp4|webm|mkv|avi|mov|flv|wmv|m4v)/i);
    return m ? ['direct', m[1].toUpperCase()] : ['direct', 'VIDEO'];
  }}

  function ok(u) {{
    if (!u || !VID.test(u)) return false;
    if (/\.ts(\?|#|$)/i.test(u) && !/[_\-](full|complete|movie|episode)/i.test(u)) return false;
    return true;
  }}

  // ── 1. DOM scanner ──
  function scanDOM() {{
    document.querySelectorAll('video').forEach(v => {{
      // Skip videos shorter than 1 second
      if (v.duration && isFinite(v.duration) && v.duration < 1) return;
      [v.src, v.currentSrc].forEach(s => {{
        if (ok(s)) {{ const [t,l] = classify(s); report(s, t, l); }}
      }});
      v.querySelectorAll('source').forEach(s => {{
        if (ok(s.src)) {{ const [t,l] = classify(s.src); report(s.src, t, l); }}
      }});
    }});
    const og = document.querySelector('meta[property="og:video"],meta[property="og:video:url"]');
    if (og && ok(og.content)) {{ const [t,l] = classify(og.content); report(og.content, t, l); }}
  }}

  // ── 2. Hook fetch ──
  const _fetch = window.fetch;
  window.fetch = async function(...args) {{
    const res = await _fetch.apply(this, args);
    try {{
      const url = typeof args[0] === 'string' ? args[0] : args[0]?.url || '';
      if (ok(url) && !isJunk(url)) {{ const [t,l] = classify(url); report(url, t, l); }}
      const ct = res.headers?.get?.('content-type') || '';
      if ((ct.startsWith('video/') || ct.includes('mpegurl') || ct.includes('dash+xml')) && !isJunk(url)) {{
        report(url, ct.includes('mpegurl') ? 'hls' : ct.includes('dash') ? 'dash' : 'direct', 'STREAM');
      }}
    }} catch(e) {{}}
    return res;
  }};

  // ── 3. Hook XHR ──
  const _open = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function(method, url, ...rest) {{
    this.__cs_url = String(url);
    try {{
      const u = this.__cs_url;
      if (ok(u) && !isJunk(u)) {{ const [t,l] = classify(u); report(u, t, l); }}
    }} catch(e) {{}}
    return _open.call(this, method, url, ...rest);
  }};

  // ── 4. MutationObserver ──
  function startObserver() {{
    if (!document.body) return;
    new MutationObserver(() => scanDOM())
      .observe(document.body, {{ childList: true, subtree: true }});
  }}
  if (document.body) startObserver();
  else document.addEventListener('DOMContentLoaded', startObserver);

  // ── 5. Periodic scans ──
  setTimeout(scanDOM, 500);
  setTimeout(scanDOM, 2000);
  setTimeout(scanDOM, 5000);
  setInterval(scanDOM, 10000);

}})();
"#,
        port = port
    )
}

// ── Commands ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn open_browser_view(
    app: AppHandle,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if let Some(wv) = app.get_webview("browser-view") {
        wv.close().map_err(|e: tauri::Error| e.to_string())?;
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }

    {
        let state = app.state::<BrowserState>();
        state.detected.lock().unwrap().clear();
    }

    let port = ensure_server(&app);
    let inject_script = make_inject_script(port);

    let app_nav = app.clone();
    let app_load = app.clone();

    let parsed_url: url::Url = url
        .parse()
        .map_err(|e| format!("Invalid URL: {}", e))?;

    let window = app
        .get_window("main")
        .ok_or("Main window not found")?;

    let builder = tauri::webview::WebviewBuilder::new(
        "browser-view",
        WebviewUrl::External(parsed_url),
    )
    .initialization_script(&inject_script)
    .auto_resize()
    .on_navigation(move |nav_url| {
        let _ = app_nav.emit("browser-url-changed", nav_url.as_str());
        true
    })
    .on_page_load(move |_wv, payload| {
        if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
            let _ = app_load.emit("browser-page-loaded", payload.url().as_str());
        }
    });

    window
        .add_child(
            builder,
            tauri::LogicalPosition::new(x, y),
            tauri::LogicalSize::new(width, height),
        )
        .map_err(|e| format!("Failed to create browser view: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn navigate_browser(app: AppHandle, url: String) -> Result<(), String> {
    {
        let state = app.state::<BrowserState>();
        state.detected.lock().unwrap().clear();
        let _ = app.emit("browser-videos-cleared", ());
    }

    if let Some(wv) = app.get_webview("browser-view") {
        let parsed: url::Url = url
            .parse()
            .map_err(|e| format!("Invalid URL: {}", e))?;
        wv.navigate(parsed).map_err(|e: tauri::Error| e.to_string())?;
    } else {
        return Err("Browser view not open".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn resize_browser(
    app: AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if let Some(wv) = app.get_webview("browser-view") {
        let _ = wv.set_position(tauri::LogicalPosition::new(x, y));
        let _ = wv.set_size(tauri::LogicalSize::new(width, height));
    }
    Ok(())
}

#[tauri::command]
pub async fn close_browser(app: AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview("browser-view") {
        wv.close().map_err(|e: tauri::Error| e.to_string())?;
    }
    let state = app.state::<BrowserState>();
    state.detected.lock().unwrap().clear();
    Ok(())
}

#[tauri::command]
pub async fn browser_go_back(app: AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview("browser-view") {
        wv.eval("window.history.back()")
            .map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn browser_go_forward(app: AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview("browser-view") {
        wv.eval("window.history.forward()")
            .map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn browser_refresh(app: AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview("browser-view") {
        wv.eval("window.location.reload()")
            .map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_detected_videos(app: AppHandle) -> Result<Vec<DetectedVideo>, String> {
    let state = app.state::<BrowserState>();
    let mut videos = state.detected.lock().unwrap().clone();
    // Sort: biggest files first, unknowns at end
    videos.sort_by(|a, b| {
        let sa = a.file_size.unwrap_or(0);
        let sb = b.file_size.unwrap_or(0);
        sb.cmp(&sa)
    });
    Ok(videos)
}

#[tauri::command]
pub async fn show_browser(app: AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview("browser-view") {
        wv.show().map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_browser(app: AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview("browser-view") {
        wv.hide().map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

/// Get all cookies from the browser webview for a specific URL.
/// Uses Tauri's native cookie API which includes HttpOnly + Secure cookies.
#[tauri::command]
pub async fn get_browser_cookies(app: AppHandle, url: String) -> Result<String, String> {
    if let Some(wv) = app.get_webview("browser-view") {
        let parsed: url::Url = url.parse().map_err(|e| format!("Bad URL: {}", e))?;
        match wv.cookies_for_url(parsed) {
            Ok(cookies) => {
                let cookie_str: String = cookies.iter()
                    .map(|c| format!("{}={}", c.name(), c.value()))
                    .collect::<Vec<_>>()
                    .join("; ");
                Ok(cookie_str)
            }
            Err(e) => {
                // Fall back to cookies stored on detected video
                let state = app.state::<BrowserState>();
                let detected = state.detected.lock().unwrap();
                if let Some(v) = detected.iter().find(|v| v.url == url || v.page_url == url) {
                    Ok(v.cookies.clone().unwrap_or_default())
                } else {
                    Err(format!("Cookie fetch failed: {}", e))
                }
            }
        }
    } else {
        Err("Browser not open".to_string())
    }
}

/// Remove a video from the detected list (after downloading)
#[tauri::command]
pub async fn remove_detected_video(app: AppHandle, url: String) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let mut detected = state.detected.lock().unwrap();
    detected.retain(|v| v.url != url);
    Ok(())
}
