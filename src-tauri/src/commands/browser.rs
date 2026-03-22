// commands/browser.rs — Built-in browser with video detection + MSE/blob capture
// Creates a child webview INSIDE the main window, injects video detection
// hooks AND MSE streaming capture, writes chunks directly to disk.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};

// ── State ────────────────────────────────────────────────────────────────────

pub struct BrowserState {
    pub server_port: Mutex<Option<u16>>,
    pub detected: Mutex<Vec<DetectedVideo>>,
    /// Active MSE captures: capture_id -> temp file path
    pub captures: Mutex<HashMap<String, CaptureState>>,
    /// Download folder for saving captured videos
    pub download_folder: Mutex<String>,
    /// Whether ad blocking is enabled in the browser
    pub adblock_enabled: Mutex<bool>,
    /// Whether popup blocking is enabled in the browser
    pub popup_blocker_enabled: Mutex<bool>,
}

pub struct CaptureState {
    pub file: File,
    pub path: String,
    pub mime_type: String,
    pub total_bytes: u64,
    pub page_title: String,
    pub page_url: String,
}

impl BrowserState {
    pub fn new() -> Self {
        let dl_folder = dirs::download_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        Self {
            server_port: Mutex::new(None),
            detected: Mutex::new(vec![]),
            captures: Mutex::new(HashMap::new()),
            download_folder: Mutex::new(dl_folder),
            adblock_enabled: Mutex::new(true),
            popup_blocker_enabled: Mutex::new(true),
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
    /// For MSE captures, this is the path to the saved file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// File size in bytes (for captures)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    /// Cookies from the browser session (for auth-gated streams)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<String>,
}

// ── Local detection + capture server ─────────────────────────────────────────

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
    let mut buf = vec![0u8; 2 * 1024 * 1024]; // 2MB read buffer
    let n = stream.read(&mut buf).unwrap_or(0);
    if n == 0 { return; }

    let request = String::from_utf8_lossy(&buf[..n]).to_string();
    let first_line = request.lines().next().unwrap_or("");

    // ── Route: GET /report?data=... (URL detection) ──
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
                    // HLS/DASH always get added (we can't HEAD-check a playlist)
                    if video.video_type == "hls" || video.video_type == "dash" {
                        detected.push(video.clone());
                        let _ = app.emit("browser-video-detected", &video);
                    } else {
                        // For direct videos, spawn a HEAD request to get size
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

    // ── Route: POST /capture-start (MSE stream started) ──
    if first_line.starts_with("POST") && first_line.contains("/capture-start") {
        if let Some(body) = extract_body(&request) {
            if let Ok(info) = serde_json::from_str::<CaptureStartInfo>(&body) {
                let state = app.state::<BrowserState>();
                let dl_folder = state.download_folder.lock().unwrap().clone();

                let ext = if info.mime_type.contains("webm") { "webm" } else { "mp4" };
                let safe_title = sanitize(&info.page_title);
                let filename = format!("{}_{}.{}", safe_title, &info.id[..6.min(info.id.len())], ext);
                let path = std::path::Path::new(&dl_folder).join(&filename);
                let path_str = path.to_string_lossy().to_string();

                match File::create(&path) {
                    Ok(file) => {
                        let mut captures = state.captures.lock().unwrap();
                        captures.insert(info.id.clone(), CaptureState {
                            file,
                            path: path_str,
                            mime_type: info.mime_type,
                            total_bytes: 0,
                            page_title: info.page_title,
                            page_url: info.page_url,
                        });
                    }
                    Err(e) => {
                        eprintln!("Failed to create capture file: {}", e);
                    }
                }
            }
        }
        send_ok(&mut stream);
        return;
    }

    // ── Route: POST /capture-chunk (MSE chunk data — binary after headers) ──
    if first_line.starts_with("POST") && first_line.contains("/capture-chunk") {
        // Extract capture ID from query string
        let capture_id = first_line
            .split("/capture-chunk?id=")
            .nth(1)
            .and_then(|s| s.split(' ').next())
            .unwrap_or("")
            .to_string();

        // Find the body (binary data after \r\n\r\n)
        if let Some(header_end) = find_header_end(&buf[..n]) {
            let body_bytes = &buf[header_end..n];

            if !capture_id.is_empty() && !body_bytes.is_empty() {
                let state = app.state::<BrowserState>();
                let mut captures = state.captures.lock().unwrap();
                if let Some(cap) = captures.get_mut(&capture_id) {
                    let _ = cap.file.write_all(body_bytes);
                    cap.total_bytes += body_bytes.len() as u64;
                }
            }
        }
        send_ok(&mut stream);
        return;
    }

    // ── Route: POST /capture-end (MSE stream finished) ──
    if first_line.starts_with("POST") && first_line.contains("/capture-end") {
        if let Some(body) = extract_body(&request) {
            if let Ok(info) = serde_json::from_str::<CaptureEndInfo>(&body) {
                let state = app.state::<BrowserState>();
                let mut captures = state.captures.lock().unwrap();
                if let Some(cap) = captures.remove(&info.id) {
                    // Flush and drop the file handle
                    drop(cap.file);

                    let video = DetectedVideo {
                        url: format!("capture://{}", info.id),
                        video_type: "capture".to_string(),
                        label: if cap.mime_type.contains("webm") { "WEBM".to_string() } else { "MP4".to_string() },
                        page_url: cap.page_url,
                        page_title: cap.page_title.clone(),
                        file_path: Some(cap.path.clone()),
                        file_size: Some(cap.total_bytes),
                        cookies: None,
                    };

                    let mut detected = state.detected.lock().unwrap();
                    detected.push(video.clone());
                    let _ = app.emit("browser-video-detected", &video);
                    let _ = app.emit("browser-capture-complete", &video);
                }
            }
        }
        send_ok(&mut stream);
        return;
    }

    // ── Route: POST /blob-save (blob video — full binary payload) ──
    if first_line.starts_with("POST") && first_line.contains("/blob-save") {
        // Extract mime and title from query
        let qs = first_line
            .split("/blob-save?")
            .nth(1)
            .and_then(|s| s.split(' ').next())
            .unwrap_or("");
        let params = parse_qs(qs);
        let mime = params.get("mime").cloned().unwrap_or_default();
        let title = params.get("title").cloned().unwrap_or_else(|| "blob_video".to_string());
        let page_url = params.get("url").cloned().unwrap_or_default();

        if let Some(header_end) = find_header_end(&buf[..n]) {
            let body_bytes = &buf[header_end..n];
            if body_bytes.len() > 1024 * 1024 { // Only save if > 1MB
                let state = app.state::<BrowserState>();
                let dl_folder = state.download_folder.lock().unwrap().clone();

                let ext = if mime.contains("webm") { "webm" } else { "mp4" };
                let safe_title = sanitize(&title);
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                let filename = format!("{}_{}.{}", safe_title, ts % 100000, ext);
                let path = std::path::Path::new(&dl_folder).join(&filename);
                let path_str = path.to_string_lossy().to_string();

                if let Ok(mut file) = File::create(&path) {
                    let _ = file.write_all(body_bytes);
                    drop(file);

                    let video = DetectedVideo {
                        url: format!("blob://{}", ts),
                        video_type: "capture".to_string(),
                        label: ext.to_uppercase(),
                        page_url,
                        page_title: title,
                        file_path: Some(path_str),
                        file_size: Some(body_bytes.len() as u64),
                        cookies: None,
                    };

                    let mut detected = state.detected.lock().unwrap();
                    detected.push(video.clone());
                    let _ = app.emit("browser-video-detected", &video);
                    let _ = app.emit("browser-capture-complete", &video);
                }
            }
        }
        send_ok(&mut stream);
        return;
    }

    // Fallback
    send_ok(&mut stream);
}

#[derive(Deserialize)]
struct CaptureStartInfo {
    id: String,
    mime_type: String,
    page_title: String,
    page_url: String,
}

#[derive(Deserialize)]
struct CaptureEndInfo {
    id: String,
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

fn extract_body(request: &str) -> Option<String> {
    request.split("\r\n\r\n").nth(1).map(|s| s.to_string())
}

fn parse_qs(qs: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in qs.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(k.to_string(), percent_decode(v));
        }
    }
    map
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

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .chars()
        .take(80)
        .collect()
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
        // For HTTPS we need TLS — use a simple reqwest blocking call
        // Fall back to spawning a quick process or just skip
        // Since we already have reqwest, use a simple approach
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
// Also hooks MSE SourceBuffer + URL.createObjectURL for blob/MSE capture.
// Streams MSE chunks directly to Rust via POST to localhost TCP server.

fn make_inject_script(port: u16, adblock_enabled: bool, popup_blocker_enabled: bool) -> String {
    format!(
        r#"
(function() {{
  'use strict';
  if (window.__cs_injected) return;
  window.__cs_injected = true;

  // ── Adblock + Popup Blocker settings (controlled from Settings UI) ──
  window.__cs_adblock_enabled = {adblock_enabled};
  window.__cs_popup_blocker_enabled = {popup_blocker_enabled};

  // ── AD BLOCKER ──────────────────────────────────────────────────────────
  const AD_DOMAINS = [
    'doubleclick.net', 'googlesyndication.com', 'googleadservices.com',
    'google-analytics.com', 'googletagmanager.com', 'googletagservices.com',
    'adservice.google.com', 'pagead2.googlesyndication.com',
    'facebook.com/tr', 'connect.facebook.net/en_US/fbevents',
    'amazon-adsystem.com', 'ads-api.twitter.com',
    'ads.yahoo.com', 'analytics.yahoo.com',
    'moatads.com', 'scorecardresearch.com',
    'outbrain.com', 'taboola.com', 'mgid.com', 'revcontent.com',
    'adnxs.com', 'adsrvr.org', 'bidswitch.net', 'casalemedia.com',
    'criteo.com', 'criteo.net', 'demdex.net', 'exelator.com',
    'eyeota.net', 'krxd.net', 'lijit.com', 'mathtag.com',
    'openx.net', 'pubmatic.com', 'rubiconproject.com',
    'sharethis.com', 'sharethrough.com', 'smartadserver.com',
    'spotxchange.com', 'teads.tv', 'yieldmo.com',
    'imasdk.googleapis.com', 'tpc.googlesyndication.com',
    'ad.doubleclick.net', 'static.doubleclick.net',
    'mediavisor.doubleclick.net',
    'quantserve.com', 'serving-sys.com', 'adtechus.com',
    'advertising.com', 'atdmt.com', 'adform.net',
    'zedo.com', 'mixpanel.com', 'hotjar.com',
    'fullstory.com', 'mouseflow.com',
  ];

  const AD_CSS_SELECTORS = [
    '[id*="google_ads"]', '[id*="ad-container"]', '[id*="ad_container"]',
    '[id*="adunit"]', '[id*="ad-unit"]', '[id*="adslot"]',
    '[class*="ad-container"]', '[class*="ad_container"]',
    '[class*="ad-wrapper"]', '[class*="ad_wrapper"]',
    '[class*="adsbygoogle"]', '[class*="ad-banner"]',
    '[class*="sponsored-content"]', '[class*="sponsored_content"]',
    'ins.adsbygoogle', 'iframe[src*="doubleclick"]',
    'iframe[src*="googlesyndication"]', 'iframe[src*="googleads"]',
    '[data-ad]', '[data-ad-slot]', '[data-ad-client]',
    '[data-google-query-id]', '[data-ad-manager-id]',
    '.ad-slot', '.ad-placement', '.ad-zone',
  ];

  let adblockStyleEl = null;

  function isAdDomain(url) {{
    try {{
      const hostname = new URL(url, location.href).hostname;
      return AD_DOMAINS.some(d => hostname.includes(d) || hostname.endsWith('.' + d));
    }} catch(e) {{
      return AD_DOMAINS.some(d => url.includes(d));
    }}
  }}

  function injectAdblockCSS() {{
    if (adblockStyleEl) return;
    adblockStyleEl = document.createElement('style');
    adblockStyleEl.id = '__cs_adblock_css';
    adblockStyleEl.textContent = AD_CSS_SELECTORS.join(',\n') + ' {{ display: none !important; visibility: hidden !important; height: 0 !important; width: 0 !important; overflow: hidden !important; }}';
    (document.head || document.documentElement).appendChild(adblockStyleEl);
  }}

  function removeAdblockCSS() {{
    if (adblockStyleEl) {{
      adblockStyleEl.remove();
      adblockStyleEl = null;
    }}
  }}

  // Expose enable/disable functions for live toggling from Rust
  window.__cs_enableAdblock = function() {{
    window.__cs_adblock_enabled = true;
    injectAdblockCSS();
  }};
  window.__cs_disableAdblock = function() {{
    window.__cs_adblock_enabled = false;
    removeAdblockCSS();
  }};

  // Inject CSS immediately if enabled
  if (window.__cs_adblock_enabled) {{
    if (document.head || document.documentElement) {{
      injectAdblockCSS();
    }} else {{
      document.addEventListener('DOMContentLoaded', () => {{
        if (window.__cs_adblock_enabled) injectAdblockCSS();
      }});
    }}
  }}

  // ── POPUP BLOCKER ───────────────────────────────────────────────────────
  const _origWindowOpen = window.open;
  window.open = function(...args) {{
    if (window.__cs_popup_blocker_enabled) {{
      console.log('[CyberSnatcher] Popup blocked:', args[0]);
      return null;
    }}
    return _origWindowOpen.apply(this, args);
  }};

  window.__cs_enablePopupBlocker = function() {{
    window.__cs_popup_blocker_enabled = true;
  }};
  window.__cs_disablePopupBlocker = function() {{
    window.__cs_popup_blocker_enabled = false;
  }};

  // ── VIDEO DETECTION (existing logic) ────────────────────────────────────
  const PORT = {port};
  const BASE = 'http://127.0.0.1:' + PORT;
  const reported = new Set();

  // ── Smart filtering patterns (ported from Video Snatcher extension) ──
  const AD_PAT = /ads?[_\-.]|track(ing)?|beacon|pixel|analytics|prebid|imasdk|doubleclick|googlesyndication|moatads|scorecardresearch/i;
  const THUMB_PAT = /thumb|preview|poster|sprite|placeholder|gif\.mp4|_default\.|rollover|\/key[s]?[\/\?]|\/key[0-9a-f]|_thumb|_small|_mini|sample|trailer_|teaser/i;
  const SKIP_EXT = /\.(php|html?|aspx?|jsp|json|js|css|png|jpe?g|gif|svg|woff2?|ico|ttf|eot|xml|m4s|m4f|cmfv|cmfa)(\?|#|$)/i;
  const SKIP_PATH = /\/(embed|watch|view_video|player|login|signup|register|checkout)\b/i;
  const TINY_PATH = /[_\-](120|160|180|240|320)p?[_\-.]|[_\-]small|[_\-]thumb|[_\-]preview|_low\b/i;

  function isJunk(url) {{
    return AD_PAT.test(url) || THUMB_PAT.test(url) || SKIP_EXT.test(url) || SKIP_PATH.test(url) || TINY_PATH.test(url);
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

  const VID = /\.(mp4|webm|mkv|avi|mov|flv|wmv|m4v|m3u8|mpd|ts)([\/\?#]|$)/i;

  function classify(url) {{
    if (/\.m3u8/i.test(url)) return ['hls', 'HLS'];
    if (/\.mpd/i.test(url)) return ['dash', 'DASH'];
    const m = url.match(/\.(mp4|webm|mkv|avi|mov|flv|wmv|m4v)/i);
    return m ? ['direct', m[1].toUpperCase()] : ['direct', 'VIDEO'];
  }}

  function ok(u) {{
    if (!u || !VID.test(u)) return false;
    // Extra check: skip .ts segments (HLS chunks, not full videos)
    if (/\.ts(\?|#|$)/i.test(u) && !/[_\-](full|complete|movie|episode)/i.test(u)) return false;
    return true;
  }}

  // ── 1. DOM scanner ──
  function scanDOM() {{
    document.querySelectorAll('video').forEach(v => {{
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

  // ── 2. Hook fetch (video detection + adblock) ──
  const _fetch = window.fetch;
  window.fetch = async function(...args) {{
    try {{
      const url = typeof args[0] === 'string' ? args[0] : args[0]?.url || '';
      // Adblock: block requests to ad domains
      if (window.__cs_adblock_enabled && isAdDomain(url)) {{
        return new Response('', {{ status: 204, statusText: 'Blocked by CyberSnatcher' }});
      }}
    }} catch(e) {{}}
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

  // ── 3. Hook XHR (video detection + adblock) ──
  const _open = XMLHttpRequest.prototype.open;
  const _xhrSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function(method, url, ...rest) {{
    this.__cs_url = String(url);
    try {{
      const u = this.__cs_url;
      if (ok(u) && !isJunk(u)) {{ const [t,l] = classify(u); report(u, t, l); }}
    }} catch(e) {{}}
    return _open.call(this, method, url, ...rest);
  }};
  XMLHttpRequest.prototype.send = function(...args) {{
    if (window.__cs_adblock_enabled && this.__cs_url && isAdDomain(this.__cs_url)) {{
      // Block ad XHR by aborting
      return;
    }}
    return _xhrSend.apply(this, args);
  }};

  // ── Adblock: Block ad script/iframe creation ──
  if (window.__cs_adblock_enabled) {{
    const _createElement = document.createElement.bind(document);
    document.createElement = function(tag, options) {{
      const el = _createElement(tag, options);
      if (window.__cs_adblock_enabled && (tag === 'script' || tag === 'iframe')) {{
        const origSetAttr = el.setAttribute.bind(el);
        el.setAttribute = function(name, value) {{
          if ((name === 'src' || name === 'href') && isAdDomain(String(value))) {{
            console.log('[CyberSnatcher] Blocked ad element:', tag, value);
            return;
          }}
          return origSetAttr(name, value);
        }};
        const srcDesc = Object.getOwnPropertyDescriptor(HTMLScriptElement.prototype, 'src') ||
                         Object.getOwnPropertyDescriptor(HTMLIFrameElement.prototype, 'src');
        if (srcDesc && srcDesc.set) {{
          const origSrcSet = srcDesc.set;
          Object.defineProperty(el, 'src', {{
            set: function(v) {{
              if (window.__cs_adblock_enabled && isAdDomain(String(v))) {{
                console.log('[CyberSnatcher] Blocked ad src:', v);
                return;
              }}
              origSrcSet.call(this, v);
            }},
            get: srcDesc.get ? srcDesc.get.bind(el) : undefined,
            configurable: true,
          }});
        }}
      }}
      return el;
    }};
  }}

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
        port = port,
        adblock_enabled = if adblock_enabled { "true" } else { "false" },
        popup_blocker_enabled = if popup_blocker_enabled { "true" } else { "false" }
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
        state.captures.lock().unwrap().clear();
    }

    let port = ensure_server(&app);
    let (adblock_on, popup_on) = {
        let state = app.state::<BrowserState>();
        (
            *state.adblock_enabled.lock().unwrap(),
            *state.popup_blocker_enabled.lock().unwrap(),
        )
    };
    let inject_script = make_inject_script(port, adblock_on, popup_on);

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
    state.captures.lock().unwrap().clear();
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

/// Get browser settings (adblock + popup blocker state)
#[tauri::command]
pub async fn get_browser_settings(app: AppHandle) -> Result<BrowserSettings, String> {
    let state = app.state::<BrowserState>();
    let adblock_enabled = *state.adblock_enabled.lock().unwrap();
    let popup_blocker_enabled = *state.popup_blocker_enabled.lock().unwrap();
    Ok(BrowserSettings {
        adblock_enabled,
        popup_blocker_enabled,
    })
}

/// Update browser settings and re-inject scripts into current webview
#[tauri::command]
pub async fn set_browser_settings(
    app: AppHandle,
    adblock_enabled: bool,
    popup_blocker_enabled: bool,
) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    *state.adblock_enabled.lock().unwrap() = adblock_enabled;
    *state.popup_blocker_enabled.lock().unwrap() = popup_blocker_enabled;

    // If the browser webview is open, inject/remove the scripts live
    if let Some(wv) = app.get_webview("browser-view") {
        let js = format!(
            "window.__cs_adblock_enabled = {}; window.__cs_popup_blocker_enabled = {};",
            adblock_enabled, popup_blocker_enabled
        );
        wv.eval(&js).map_err(|e| e.to_string())?;

        if adblock_enabled {
            wv.eval("if (typeof window.__cs_enableAdblock === 'function') window.__cs_enableAdblock();")
                .map_err(|e| e.to_string())?;
        } else {
            wv.eval("if (typeof window.__cs_disableAdblock === 'function') window.__cs_disableAdblock();")
                .map_err(|e| e.to_string())?;
        }
        if popup_blocker_enabled {
            wv.eval("if (typeof window.__cs_enablePopupBlocker === 'function') window.__cs_enablePopupBlocker();")
                .map_err(|e| e.to_string())?;
        } else {
            wv.eval("if (typeof window.__cs_disablePopupBlocker === 'function') window.__cs_disablePopupBlocker();")
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSettings {
    pub adblock_enabled: bool,
    pub popup_blocker_enabled: bool,
}
