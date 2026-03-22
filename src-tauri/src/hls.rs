// hls.rs — HLS Parser + Concurrent Segment Downloader + AES-128 Decryption
// Parses master/media playlists, downloads segments with 6 workers,
// decrypts AES-128-CBC if needed, concatenates to .ts, then remuxes via ffmpeg.

use aes::Aes128;
use cbc::{Decryptor, cipher::{BlockDecryptMut, KeyIvInit}};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex as TokioMutex;

use crate::types::DownloadProgress;

type Aes128CbcDec = Decryptor<Aes128>;

const CONCURRENCY: usize = 6;
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 800;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlsQuality {
    pub url: String,
    pub bandwidth: u64,
    pub label: String,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlsParseResult {
    pub is_master: bool,
    pub qualities: Vec<HlsQuality>,
    /// If it's a media playlist directly, this has segment info
    pub media_info: Option<HlsMediaInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlsMediaInfo {
    pub segments: usize,
    pub duration: f64,
    pub is_live: bool,
    pub encrypted: bool,
}

#[derive(Debug, Clone)]
struct HlsSegment {
    url: String,
    seq: u64,
    duration: f64,
    key: Option<HlsKey>,
}

#[derive(Debug, Clone)]
struct HlsKey {
    uri: String,
    iv: Option<[u8; 16]>,
}

#[derive(Debug, Clone)]
struct HlsMediaPlaylist {
    segments: Vec<HlsSegment>,
    total_duration: f64,
    is_live: bool,
    init_map_url: Option<String>,
    encryption: Option<HlsKey>,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Parse an HLS URL — returns qualities if master, or media info if direct
pub async fn parse_hls_url(url: &str) -> Result<HlsParseResult, String> {
    let client = build_client();
    let text = fetch_text(&client, url).await?;
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    let is_master = lines.iter().any(|l| l.contains("#EXT-X-STREAM-INF"));

    if is_master {
        let qualities = parse_master_playlist(&lines, url);
        Ok(HlsParseResult {
            is_master: true,
            qualities,
            media_info: None,
        })
    } else {
        let media = parse_media_playlist(&lines, url);
        Ok(HlsParseResult {
            is_master: false,
            qualities: vec![HlsQuality {
                url: url.to_string(),
                bandwidth: 0,
                label: "Default".to_string(),
                resolution: None,
            }],
            media_info: Some(HlsMediaInfo {
                segments: media.segments.len(),
                duration: media.total_duration,
                is_live: media.is_live,
                encrypted: media.encryption.is_some(),
            }),
        })
    }
}

/// Download an HLS stream — resolves master if needed, downloads all segments,
/// decrypts if encrypted, concatenates, then remuxes to MP4 via ffmpeg.
pub async fn download_hls(
    app: &AppHandle,
    job_id: &str,
    url: &str,
    output_dir: &str,
    filename: &str,
    quality_idx: Option<usize>,
    cookies: Option<&str>,
    page_url: Option<&str>,
    cancelled: &Arc<AtomicBool>,
) -> Result<String, String> {
    let client = build_client_with_cookies(cookies, page_url);

    emit_progress(app, job_id, 0.0, "downloading", "Parsing HLS manifest...");

    // Step 1: fetch and parse
    let text = fetch_text(&client, url).await?;
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let is_master = lines.iter().any(|l| l.contains("#EXT-X-STREAM-INF"));

    let media_url;
    if is_master {
        let qualities = parse_master_playlist(&lines, url);
        if qualities.is_empty() {
            return Err("No qualities found in master playlist".to_string());
        }
        let idx = quality_idx.unwrap_or(qualities.len() - 1).min(qualities.len() - 1);
        media_url = qualities[idx].url.clone();
        emit_progress(app, job_id, 2.0, "downloading",
            &format!("Selected quality: {}", qualities[idx].label));
    } else {
        media_url = url.to_string();
    }

    // Step 2: parse media playlist
    let media_text = if media_url == url {
        text.clone()
    } else {
        fetch_text(&client, &media_url).await?
    };
    let media_lines: Vec<&str> = media_text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let playlist = parse_media_playlist(&media_lines, &media_url);

    if playlist.segments.is_empty() {
        return Err("No segments found in playlist".to_string());
    }

    emit_progress(app, job_id, 5.0, "downloading",
        &format!("{} segments, {:.0}s duration", playlist.segments.len(), playlist.total_duration));

    // Step 3: fetch fMP4 init segment if present
    let mut init_data: Option<Vec<u8>> = None;
    if let Some(ref init_url) = playlist.init_map_url {
        emit_progress(app, job_id, 6.0, "downloading", "Fetching init segment...");
        match fetch_bytes(&client, init_url).await {
            Ok(data) => { init_data = Some(data); }
            Err(e) => { emit_progress(app, job_id, 6.0, "downloading", &format!("Init segment error: {}", e)); }
        }
    }

    // Step 4: fetch decryption key(s) if encrypted (supports key rotation)
    let key_cache: Arc<TokioMutex<std::collections::HashMap<String, Vec<u8>>>> =
        Arc::new(TokioMutex::new(std::collections::HashMap::new()));
    let mut default_iv: Option<[u8; 16]> = None;
    if let Some(ref key_info) = playlist.encryption {
        emit_progress(app, job_id, 7.0, "downloading", "Fetching decryption key...");
        let key_bytes = fetch_bytes(&client, &key_info.uri).await
            .map_err(|e| format!("Failed to fetch key: {}", e))?;
        if key_bytes.len() != 16 {
            return Err(format!("Key size {} != 16", key_bytes.len()));
        }
        key_cache.lock().await.insert(key_info.uri.clone(), key_bytes);
        default_iv = key_info.iv;
        emit_progress(app, job_id, 8.0, "downloading", "Decryption key loaded");
    }

    // Step 5: download all segments concurrently
    let total_segs = playlist.segments.len();
    let segments = Arc::new(playlist.segments);
    let next_idx = Arc::new(AtomicUsize::new(0));
    let done_count = Arc::new(AtomicUsize::new(0));
    let failed_count = Arc::new(AtomicUsize::new(0));
    let results: Arc<TokioMutex<Vec<Option<Vec<u8>>>>> =
        Arc::new(TokioMutex::new(vec![None; total_segs]));

    let worker_count = CONCURRENCY.min(total_segs);

    emit_progress(app, job_id, 10.0, "downloading",
        &format!("Downloading {} segments ({} workers)...", total_segs, worker_count));

    let mut handles = Vec::new();

    for _ in 0..worker_count {
        let client = client.clone();
        let segments = segments.clone();
        let next_idx = next_idx.clone();
        let done_count = done_count.clone();
        let failed_count = failed_count.clone();
        let results = results.clone();
        let cancelled = cancelled.clone();
        let key_cache = key_cache.clone();
        let default_iv = default_iv;
        let app = app.clone();
        let job_id = job_id.to_string();
        let total = total_segs;

        handles.push(tokio::spawn(async move {
            loop {
                let idx = next_idx.fetch_add(1, Ordering::SeqCst);
                if idx >= total { break; }
                if cancelled.load(Ordering::Relaxed) { break; }

                let seg = &segments[idx];
                let mut data: Option<Vec<u8>> = None;

                for attempt in 1..=MAX_RETRIES {
                    match fetch_bytes(&client, &seg.url).await {
                        Ok(mut bytes) => {
                            // Decrypt if needed (supports key rotation)
                            if let Some(ref seg_key) = seg.key {
                                let key_uri = &seg_key.uri;
                                // Fetch key if not cached yet
                                let key_bytes = {
                                    let cache = key_cache.lock().await;
                                    cache.get(key_uri).cloned()
                                };
                                let key = match key_bytes {
                                    Some(k) => k,
                                    None => {
                                        match fetch_bytes(&client, key_uri).await {
                                            Ok(k) if k.len() == 16 => {
                                                key_cache.lock().await.insert(key_uri.clone(), k.clone());
                                                k
                                            }
                                            _ => { data = None; break; }
                                        }
                                    }
                                };
                                let iv = seg_key.iv
                                    .or(default_iv)
                                    .unwrap_or_else(|| sequence_iv(seg.seq));
                                match decrypt_aes128(&bytes, &key, &iv) {
                                    Ok(decrypted) => { bytes = decrypted; }
                                    Err(_) => { data = None; break; }
                                }
                            }
                            data = Some(bytes);
                            break;
                        }
                        Err(e) => {
                            if attempt < MAX_RETRIES {
                                tokio::time::sleep(std::time::Duration::from_millis(
                                    RETRY_DELAY_MS * attempt as u64
                                )).await;
                            } else {
                                eprintln!("Seg {} failed after {} retries: {}", idx, MAX_RETRIES, e);
                            }
                        }
                    }
                }

                if let Some(d) = data {
                    let mut res = results.lock().await;
                    res[idx] = Some(d);
                    done_count.fetch_add(1, Ordering::SeqCst);
                } else {
                    failed_count.fetch_add(1, Ordering::SeqCst);
                }

                // Report progress
                let done = done_count.load(Ordering::SeqCst);
                let pct = 10.0 + (done as f64 / total as f64) * 80.0; // 10% to 90%
                let _ = app.emit("download-progress", DownloadProgress {
                    job_id: job_id.clone(),
                    percent: pct,
                    speed: String::new(),
                    eta: String::new(),
                    status: "downloading".to_string(),
                    log_line: format!("Segment {}/{}", done, total),
                    file_path: None,
                    file_size: None,
                });
            }
        }));
    }

    // Wait for all workers
    for h in handles {
        let _ = h.await;
    }

    if cancelled.load(Ordering::Relaxed) {
        return Err("Cancelled".to_string());
    }

    let done = done_count.load(Ordering::SeqCst);
    let failed = failed_count.load(Ordering::SeqCst);

    if done == 0 {
        return Err("All segments failed to download".to_string());
    }

    emit_progress(app, job_id, 90.0, "downloading",
        &format!("{} segments downloaded, {} failed", done, failed));

    // Step 6: concatenate segments into a temp .ts file
    emit_progress(app, job_id, 92.0, "converting", "Assembling segments...");

    let safe_name = crate::ytdlp::sanitize_filename(filename);
    let ts_path = PathBuf::from(output_dir).join(format!("{}.ts", safe_name));
    let mp4_path = PathBuf::from(output_dir).join(format!("{}.mp4", safe_name));

    {
        let mut file = std::fs::File::create(&ts_path)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        // Write init segment first if fMP4
        if let Some(ref init) = init_data {
            std::io::Write::write_all(&mut file, init)
                .map_err(|e| format!("Write init error: {}", e))?;
        }

        let res = results.lock().await;
        for (i, seg_data) in res.iter().enumerate() {
            if let Some(data) = seg_data {
                std::io::Write::write_all(&mut file, data)
                    .map_err(|e| format!("Write seg {} error: {}", i, e))?;
            }
        }
    }

    let ts_size = std::fs::metadata(&ts_path).map(|m| m.len()).unwrap_or(0);
    emit_progress(app, job_id, 94.0, "converting",
        &format!("Raw stream: {} bytes, remuxing to MP4...", ts_size));

    // Step 7: remux TS → MP4 via ffmpeg
    let ffmpeg_result = remux_to_mp4(app, &ts_path, &mp4_path).await;

    match ffmpeg_result {
        Ok(_) => {
            // Only delete the temp .ts once we know ffmpeg succeeded
            let _ = std::fs::remove_file(&ts_path);

            let mp4_size = std::fs::metadata(&mp4_path).map(|m| m.len()).unwrap_or(0);
            let mp4_str = mp4_path.to_string_lossy().to_string();

            emit_progress(app, job_id, 100.0, "complete",
                &format!("HLS download complete: {}", mp4_str));

            let _ = app.emit("download-progress", DownloadProgress {
                job_id: job_id.to_string(),
                percent: 100.0,
                speed: String::new(),
                eta: String::new(),
                status: "complete".to_string(),
                log_line: "EXTRACTION COMPLETE ✓".to_string(),
                file_path: Some(mp4_str.clone()),
                file_size: Some(mp4_size),
            });

            Ok(mp4_str)
        }
        Err(e) => {
            // Leave the .ts on disk so the user still has their data.
            // Return its path as a fallback so the caller can surface it.
            let ts_str = ts_path.to_string_lossy().to_string();
            Err(format!(
                "ffmpeg remux failed: {}. Raw stream preserved at: {}",
                e, ts_str
            ))
        }
    }
}

// ── Parsing ──────────────────────────────────────────────────────────────────

fn parse_master_playlist(lines: &[&str], base_url: &str) -> Vec<HlsQuality> {
    let mut qualities = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        if lines[i].contains("#EXT-X-STREAM-INF") {
            let line = lines[i];
            let bandwidth = extract_attr(line, "BANDWIDTH")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            let resolution = extract_attr(line, "RESOLUTION");
            let label = if let Some(ref res) = resolution {
                let parts: Vec<&str> = res.split('x').collect();
                if parts.len() == 2 { format!("{}p", parts[1]) }
                else { format!("{}k", bandwidth / 1000) }
            } else {
                format!("{}k", bandwidth / 1000)
            };

            // Find the URL on the next non-comment line
            let mut url_str = String::new();
            for j in (i + 1)..lines.len() {
                if !lines[j].starts_with('#') {
                    url_str = lines[j].to_string();
                    break;
                }
            }

            if !url_str.is_empty() {
                let full_url = resolve_url(base_url, &url_str);
                qualities.push(HlsQuality {
                    url: full_url,
                    bandwidth,
                    label,
                    resolution,
                });
            }
        }
        i += 1;
    }

    qualities.sort_by_key(|q| q.bandwidth);
    qualities
}

fn parse_media_playlist(lines: &[&str], base_url: &str) -> HlsMediaPlaylist {
    let mut segments = Vec::new();
    let mut current_key: Option<HlsKey> = None;
    let mut first_encryption: Option<HlsKey> = None;
    let mut media_seq: u64 = 0;
    let mut seg_idx: u64 = 0;
    let mut total_duration: f64 = 0.0;
    let mut init_map_url: Option<String> = None;
    let mut current_duration: f64 = 0.0;

    // Parse media sequence
    for line in lines {
        if line.contains("#EXT-X-MEDIA-SEQUENCE") {
            if let Some(val) = line.split(':').nth(1) {
                media_seq = val.trim().parse().unwrap_or(0);
            }
        }
    }

    for line in lines {
        // EXT-X-MAP (fMP4 init segment)
        if line.contains("#EXT-X-MAP") {
            if let Some(uri) = extract_quoted_attr(line, "URI") {
                init_map_url = Some(resolve_url(base_url, &uri));
            }
            continue;
        }

        // EXT-X-KEY
        if line.contains("#EXT-X-KEY") {
            let method = extract_attr(line, "METHOD").unwrap_or_default();
            if method == "NONE" {
                current_key = None;
            } else if method == "AES-128" {
                let uri = extract_quoted_attr(line, "URI").unwrap_or_default();
                let iv = extract_attr(line, "IV")
                    .and_then(|v| parse_iv_hex(&v));
                let key = HlsKey {
                    uri: resolve_url(base_url, &uri),
                    iv,
                };
                if first_encryption.is_none() {
                    first_encryption = Some(key.clone());
                }
                current_key = Some(key);
            }
            continue;
        }

        // EXTINF
        if line.starts_with("#EXTINF:") {
            if let Some(dur_str) = line.strip_prefix("#EXTINF:") {
                let dur_str = dur_str.split(',').next().unwrap_or("0");
                current_duration = dur_str.parse().unwrap_or(0.0);
                total_duration += current_duration;
            }
            continue;
        }

        // Skip other comments
        if line.starts_with('#') { continue; }

        // This is a segment URL
        let seg_url = resolve_url(base_url, line);
        segments.push(HlsSegment {
            url: seg_url,
            seq: media_seq + seg_idx,
            duration: current_duration,
            key: current_key.clone(),
        });
        seg_idx += 1;
        current_duration = 0.0;
    }

    let has_endlist = lines.iter().any(|l| l.contains("#EXT-X-ENDLIST"));
    let has_vod = lines.iter().any(|l| l.contains("#EXT-X-PLAYLIST-TYPE:VOD"));

    // Per HLS spec (RFC 8216 §4.3.3.4): absence of #EXT-X-ENDLIST is the
    // authoritative signal that a playlist is live.  Duration/segment-count
    // heuristics cause false positives (short VODs) and false negatives
    // (long-running live streams), so they are intentionally omitted.
    let is_live = !has_endlist && !has_vod;

    HlsMediaPlaylist {
        segments,
        total_duration,
        is_live,
        init_map_url,
        encryption: first_encryption,
    }
}

// ── AES-128-CBC Decryption ───────────────────────────────────────────────────

fn decrypt_aes128(data: &[u8], key: &[u8], iv: &[u8; 16]) -> Result<Vec<u8>, String> {
    if key.len() != 16 {
        return Err(format!("Key length {} != 16", key.len()));
    }
    if data.is_empty() {
        return Ok(vec![]);
    }

    // AES-128-CBC requires data to be multiple of 16 bytes
    // HLS segments should already be padded (PKCS7)
    let key_arr: [u8; 16] = key.try_into().map_err(|_| "Key conversion failed")?;

    let mut buf = data.to_vec();
    let decryptor = Aes128CbcDec::new(&key_arr.into(), iv.into());

    match decryptor.decrypt_padded_mut::<block_padding::Pkcs7>(&mut buf) {
        Ok(plaintext) => Ok(plaintext.to_vec()),
        Err(_) => {
            // Try without padding removal (some streams don't use PKCS7)
            let mut buf2 = data.to_vec();
            // Pad to block size if needed
            let pad_len = (16 - (buf2.len() % 16)) % 16;
            buf2.extend(std::iter::repeat(0u8).take(pad_len));

            let decryptor2 = Aes128CbcDec::new(&key_arr.into(), iv.into());
            match decryptor2.decrypt_padded_mut::<block_padding::NoPadding>(&mut buf2) {
                Ok(plaintext) => Ok(plaintext.to_vec()),
                Err(e) => Err(format!("Decryption failed: {:?}", e)),
            }
        }
    }
}

fn sequence_iv(seq: u64) -> [u8; 16] {
    let mut iv = [0u8; 16];
    iv[8] = ((seq >> 56) & 0xff) as u8;
    iv[9] = ((seq >> 48) & 0xff) as u8;
    iv[10] = ((seq >> 40) & 0xff) as u8;
    iv[11] = ((seq >> 32) & 0xff) as u8;
    iv[12] = ((seq >> 24) & 0xff) as u8;
    iv[13] = ((seq >> 16) & 0xff) as u8;
    iv[14] = ((seq >> 8) & 0xff) as u8;
    iv[15] = (seq & 0xff) as u8;
    iv
}

fn parse_iv_hex(s: &str) -> Option<[u8; 16]> {
    let hex = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    let padded = format!("{:0>32}", hex);
    let mut iv = [0u8; 16];
    for i in 0..16 {
        iv[i] = u8::from_str_radix(&padded[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(iv)
}

// ── ffmpeg remux ─────────────────────────────────────────────────────────────

async fn remux_to_mp4(app: &AppHandle, input: &PathBuf, output: &PathBuf) -> Result<(), String> {
    let ffmpeg_bin = crate::ffmpeg::resolve_ffmpeg_path(app)?;

    let output_result = tokio::process::Command::new(&ffmpeg_bin)
        .args([
            "-i", &input.to_string_lossy(),
            "-c", "copy",
            "-movflags", "+faststart",
            "-y",
            &output.to_string_lossy(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

    if output_result.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        let last_lines: String = stderr.lines().rev().take(5).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
        Err(format!("ffmpeg exited with code: {}. {}", output_result.status, last_lines))
    }
}

// ── HTTP helpers ─────────────────────────────────────────────────────────────

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .unwrap_or_default()
}

fn build_client_with_cookies(cookies: Option<&str>, page_url: Option<&str>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");

    let mut headers = reqwest::header::HeaderMap::new();

    if let Some(cookie_str) = cookies {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(cookie_str) {
            headers.insert(reqwest::header::COOKIE, val);
        }
    }

    if let Some(purl) = page_url {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(purl) {
            headers.insert(reqwest::header::REFERER, val);
        }
        // Origin = scheme + host (no path)
        if let Ok(parsed) = url::Url::parse(purl) {
            let origin = format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""));
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&origin) {
                headers.insert(reqwest::header::ORIGIN, val);
            }
        }
    }

    if !headers.is_empty() {
        builder = builder.default_headers(headers);
    }

    builder.build().unwrap_or_default()
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let resp = client.get(url).send().await
        .map_err(|e| format!("Fetch failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.text().await
        .map_err(|e| format!("Read failed: {}", e))
}

async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client.get(url).send().await
        .map_err(|e| format!("Fetch failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.bytes().await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Read failed: {}", e))
}

// ── URL + string helpers ─────────────────────────────────────────────────────

fn resolve_url(base: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }
    match url::Url::parse(base) {
        Ok(base_url) => {
            match base_url.join(relative) {
                Ok(resolved) => resolved.to_string(),
                Err(_) => relative.to_string(),
            }
        }
        Err(_) => relative.to_string(),
    }
}

fn extract_attr(line: &str, name: &str) -> Option<String> {
    let search = format!("{}=", name);
    // Require the match to be at the start of the attribute list or immediately
    // after a comma, so that "IV=" does not substring-match inside "KEYFORMATVERSIONS=".
    let pos = line.find(&search).filter(|&p| p == 0 || line.as_bytes()[p - 1] == b',')?;
    let start = pos + search.len();
    let rest = &line[start..];
    // Handle quoted values
    if rest.starts_with('"') {
        let end = rest[1..].find('"').map(|p| p + 1).unwrap_or(rest.len());
        Some(rest[1..end].to_string())
    } else {
        let end = rest.find(',').or_else(|| rest.find(' ')).unwrap_or(rest.len());
        Some(rest[..end].to_string())
    }
}

fn extract_quoted_attr(line: &str, name: &str) -> Option<String> {
    let search = format!("{}=\"", name);
    if let Some(pos) = line.find(&search) {
        let start = pos + search.len();
        let rest = &line[start..];
        let end = rest.find('"').unwrap_or(rest.len());
        Some(rest[..end].to_string())
    } else {
        None
    }
}

fn emit_progress(app: &AppHandle, job_id: &str, percent: f64, status: &str, log_line: &str) {
    let _ = app.emit("download-progress", DownloadProgress {
        job_id: job_id.to_string(),
        percent,
        speed: String::new(),
        eta: String::new(),
        status: status.to_string(),
        log_line: log_line.to_string(),
        file_path: None,
        file_size: None,
    });
}

// ══════════════════════════════════════════════════════════════════════════════
// HLS Live Stream Recording — polls manifest, grabs new segments until stopped
// ══════════════════════════════════════════════════════════════════════════════

/// Record an HLS live stream — polls manifest for new segments, writes to disk.
/// Stops when cancelled flag is set or stream ends naturally (EXT-X-ENDLIST).
pub async fn download_hls_live(
    app: &AppHandle,
    job_id: &str,
    url: &str,
    output_dir: &str,
    filename: &str,
    quality_idx: Option<usize>,
    cookies: Option<&str>,
    page_url: Option<&str>,
    cancelled: &Arc<AtomicBool>,
) -> Result<String, String> {
    let client = build_client_with_cookies(cookies, page_url);

    emit_progress(app, job_id, 0.0, "downloading", "Connecting to live stream...");

    // Resolve master playlist if needed
    let text = fetch_text(&client, url).await?;
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let is_master = lines.iter().any(|l| l.contains("#EXT-X-STREAM-INF"));

    let media_url = if is_master {
        let qualities = parse_master_playlist(&lines, url);
        if qualities.is_empty() {
            return Err("No qualities in master playlist".to_string());
        }
        let idx = quality_idx.unwrap_or(qualities.len() - 1).min(qualities.len() - 1);
        emit_progress(app, job_id, 1.0, "downloading",
            &format!("Live quality: {}", qualities[idx].label));
        qualities[idx].url.clone()
    } else {
        url.to_string()
    };

    let mut seen_seqs: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut all_buffers: Vec<Vec<u8>> = Vec::new();
    let mut crypto_key: Option<Vec<u8>> = None;
    let mut default_iv: Option<[u8; 16]> = None;
    let mut crypto_setup = false;
    let mut total_segs: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut init_data: Option<Vec<u8>> = None;
    let recording_start = std::time::Instant::now();

    emit_progress(app, job_id, 2.0, "downloading", "Recording — stop when ready");

    loop {
        if cancelled.load(Ordering::Relaxed) { break; }

        // Fetch and parse manifest
        let manifest_text = match fetch_text(&client, &media_url).await {
            Ok(t) => t,
            Err(e) => {
                emit_progress(app, job_id, -1.0, "downloading",
                    &format!("Manifest fetch error: {}, retrying...", e));
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                continue;
            }
        };

        let manifest_lines: Vec<&str> = manifest_text.lines()
            .map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        let playlist = parse_media_playlist(&manifest_lines, &media_url);

        // Fetch init segment once
        if init_data.is_none() {
            if let Some(ref init_url) = playlist.init_map_url {
                if let Ok(data) = fetch_bytes(&client, init_url).await {
                    init_data = Some(data);
                }
            }
        }

        // Setup crypto once
        if !crypto_setup {
            if let Some(ref key_info) = playlist.encryption {
                if let Ok(key_bytes) = fetch_bytes(&client, &key_info.uri).await {
                    if key_bytes.len() == 16 {
                        crypto_key = Some(key_bytes);
                        default_iv = key_info.iv;
                        crypto_setup = true;
                        emit_progress(app, job_id, -1.0, "downloading", "Decryption key loaded");
                    }
                }
            }
        }

        // Download new segments only
        let new_segs: Vec<&HlsSegment> = playlist.segments.iter()
            .filter(|s| !seen_seqs.contains(&s.seq))
            .collect();

        for seg in &new_segs {
            if cancelled.load(Ordering::Relaxed) { break; }
            seen_seqs.insert(seg.seq);

            for attempt in 1..=MAX_RETRIES {
                match fetch_bytes(&client, &seg.url).await {
                    Ok(mut bytes) => {
                        // Decrypt if needed
                        if let Some(ref key) = crypto_key {
                            let iv = seg.key.as_ref()
                                .and_then(|k| k.iv)
                                .or(default_iv)
                                .unwrap_or_else(|| sequence_iv(seg.seq));
                            if let Ok(decrypted) = decrypt_aes128(&bytes, key, &iv) {
                                bytes = decrypted;
                            }
                        }
                        total_bytes += bytes.len() as u64;
                        total_segs += 1;
                        all_buffers.push(bytes);
                        break;
                    }
                    Err(_) if attempt < MAX_RETRIES => {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            RETRY_DELAY_MS * attempt as u64
                        )).await;
                    }
                    Err(e) => {
                        eprintln!("Live seg {} failed: {}", seg.seq, e);
                    }
                }
            }

            // Update UI
            let elapsed = recording_start.elapsed().as_secs();
            let size_mb = total_bytes as f64 / 1048576.0;
            emit_progress(app, job_id, -1.0, "downloading",
                &format!("Recording: {} segs | {:.1} MB | {}s",
                    total_segs, size_mb, elapsed));
        }

        // Check if stream ended
        if !playlist.is_live {
            emit_progress(app, job_id, -1.0, "downloading", "Stream ended (EXT-X-ENDLIST)");
            break;
        }

        if !cancelled.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    if all_buffers.is_empty() {
        return Err("No segments recorded".to_string());
    }

    let actual_duration = recording_start.elapsed().as_secs_f64();

    emit_progress(app, job_id, 90.0, "converting",
        &format!("Recording stopped. {} segments, {:.1} MB",
            total_segs, total_bytes as f64 / 1048576.0));

    // Concatenate and remux
    let safe_name = crate::ytdlp::sanitize_filename(filename);
    let ts_path = PathBuf::from(output_dir).join(format!("{}_live.ts", safe_name));
    let mp4_path = PathBuf::from(output_dir).join(format!("{}.mp4", safe_name));

    {
        let mut file = std::fs::File::create(&ts_path)
            .map_err(|e| format!("Create file: {}", e))?;

        if let Some(ref init) = init_data {
            std::io::Write::write_all(&mut file, init)
                .map_err(|e| format!("Write init: {}", e))?;
        }

        for buf in &all_buffers {
            std::io::Write::write_all(&mut file, buf)
                .map_err(|e| format!("Write seg: {}", e))?;
        }
    }

    emit_progress(app, job_id, 94.0, "converting", "Remuxing to MP4...");

    let ffmpeg_result = remux_to_mp4(app, &ts_path, &mp4_path).await;

    match ffmpeg_result {
        Ok(_) => {
            // Only delete the temp .ts once we know ffmpeg succeeded
            let _ = std::fs::remove_file(&ts_path);

            // Patch duration
            crate::mp4patch::patch_mp4_duration(&mp4_path, actual_duration);

            let mp4_str = mp4_path.to_string_lossy().to_string();
            let mp4_size = std::fs::metadata(&mp4_path).map(|m| m.len()).ok();

            let _ = app.emit("download-progress", DownloadProgress {
                job_id: job_id.to_string(),
                percent: 100.0,
                speed: String::new(),
                eta: String::new(),
                status: "complete".to_string(),
                log_line: "LIVE RECORDING COMPLETE".to_string(),
                file_path: Some(mp4_str.clone()),
                file_size: mp4_size,
            });

            Ok(mp4_str)
        }
        Err(e) => {
            // Leave the .ts on disk so the user still has their recording.
            let ts_str = ts_path.to_string_lossy().to_string();
            Err(format!(
                "ffmpeg remux failed: {}. Raw stream preserved at: {}",
                e, ts_str
            ))
        }
    }
}
