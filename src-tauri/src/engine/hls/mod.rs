use obfstr::obfstr;

pub mod parser;
pub mod crypto;
pub mod live;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

use super::http::VideoClient;
use crate::types::DownloadProgress;

pub async fn download_hls(
    app: &tauri::AppHandle,
    job_id: &str,
    manifest_url: &str,
    page_url: Option<&str>,
    cookies: Option<&str>,
    output_dir: &str,
    filename: &str,
    cancelled: &AtomicBool,
) -> Result<String, String> {
    let mut cb = VideoClient::new();
    if let Some(page) = page_url { cb = cb.with_referer(page); }
    else { cb = cb.with_referer(manifest_url); }
    if let Some(c) = cookies { cb = cb.with_cookies(c); }
    let client = Arc::new(cb);

    // Emit starting
    let _ = app.emit("download-progress", DownloadProgress {
        job_id: job_id.to_string(), percent: 0.0, speed: obfstr!("Fetching manifest...").to_string(),
        eta: "\u{2014}".to_string(), status: obfstr!("downloading").to_string(),
        log_line: format!("{}{}", obfstr!("HLS: Fetching manifest "), manifest_url),
        file_path: None, file_size: None,
    });

    let manifest_text = client.get_text(manifest_url).await?;
    let parsed = parser::parse_m3u8(&manifest_text, manifest_url)?;

    let (media_playlist, _variant_urls) = match parsed {
        parser::M3u8Result::Master(master) => {
            // Try best quality first, with fallback to lower qualities
            let variant_urls: Vec<String> = master.variants.iter().map(|v| v.url.clone()).collect();
            let best = master.variants.last().ok_or(obfstr!("No variants in master playlist"))?;
            let _ = app.emit("download-progress", DownloadProgress {
                job_id: job_id.to_string(), percent: 0.0, speed: String::new(),
                eta: String::new(), status: obfstr!("downloading").to_string(),
                log_line: format!("{}{}{}{}{}",
                    obfstr!("HLS: Selected quality "),
                    best.label,
                    obfstr!(" ("),
                    best.bandwidth,
                    obfstr!("bps)")),
                file_path: None, file_size: None,
            });
            let media_text = client.get_text(&best.url).await?;
            match parser::parse_m3u8(&media_text, &best.url)? {
                parser::M3u8Result::Media(m) => (m, variant_urls),
                _ => return Err(obfstr!("Expected media playlist from variant").into()),
            }
        }
        parser::M3u8Result::Media(media) => (media, vec![]),
    };

    if media_playlist.is_live {
        return live::record_live(app, job_id, manifest_url, &client, &media_playlist, output_dir, filename, cancelled).await;
    }

    let _ = app.emit("download-progress", DownloadProgress {
        job_id: job_id.to_string(), percent: 0.0, speed: String::new(),
        eta: String::new(), status: obfstr!("downloading").to_string(),
        log_line: format!("{}{}{}{}{}{}{}{}{}{}",
            obfstr!("HLS: "),
            media_playlist.segments.len(),
            obfstr!(" segments, "),
            format!("{:.0}", media_playlist.total_duration),
            obfstr!("s, encrypted="),
            media_playlist.encryption.is_some(),
            obfstr!(", byterange="),
            media_playlist.segments.iter().any(|s| s.byterange.is_some()),
            obfstr!(", discontinuities="),
            media_playlist.has_discontinuity,
        ),
        file_path: None, file_size: None,
    });

    // Decryption setup
    let decryptor = if let Some(ref enc) = media_playlist.encryption {
        if enc.method == obfstr!("AES-128") {
            Some(crypto::HlsDecryptor::new(&client, &enc.key_uri, enc.iv.clone()).await?)
        } else if enc.method == obfstr!("SAMPLE-AES") || enc.method == obfstr!("SAMPLE-AES-CTR") {
            let _ = app.emit("download-progress", DownloadProgress {
                job_id: job_id.to_string(), percent: 0.0, speed: String::new(),
                eta: String::new(), status: obfstr!("downloading").to_string(),
                log_line: format!("{}{}{}", obfstr!("HLS: WARNING \u{2014} "), enc.method, obfstr!(" encryption detected. Segments will be downloaded raw (may not play correctly).")),
                file_path: None, file_size: None,
            });
            None
        } else {
            return Err(format!("{}{}", obfstr!("Unsupported encryption: "), enc.method));
        }
    } else { None };

    // Init segment for fMP4
    let init_segment = if let Some(ref init_url) = media_playlist.init_map_url {
        if let Some(ref br) = media_playlist.init_map_byterange {
            // Byterange init segment
            Some(client.get_bytes_range(init_url, br.offset.unwrap_or(0), br.length).await?)
        } else {
            Some(client.get_bytes(init_url).await?)
        }
    } else { None };

    // Download segments (with byterange support)
    let has_byteranges = media_playlist.segments.iter().any(|s| s.byterange.is_some());

    let results = if has_byteranges {
        // Download byterange segments sequentially or with limited concurrency
        download_byterange_segments(app, job_id, &client, &media_playlist.segments, cancelled).await?
    } else {
        let seg_urls: Vec<String> = media_playlist.segments.iter().map(|s| s.url.clone()).collect();
        super::download::download_segments(app, job_id, &client, &seg_urls, 8, cancelled).await?
    };

    if cancelled.load(Ordering::Relaxed) { return Err(obfstr!("Cancelled").into()); }

    // Decrypt
    let mut final_segments: Vec<Vec<u8>> = vec![];
    for (i, seg_data) in results.into_iter().enumerate() {
        if let Some(data) = seg_data {
            let d = if let Some(ref dec) = decryptor {
                let seg = &media_playlist.segments[i];
                dec.decrypt(&data, seg.iv.as_deref(), seg.sequence)?
            } else { data };
            final_segments.push(d);
        }
    }

    if final_segments.is_empty() { return Err(obfstr!("All segments failed").into()); }

    // Write to file
    let is_fmp4 = init_segment.is_some()
        || final_segments.first().map(|d| d.len() >= 8 && is_mp4_box(d)).unwrap_or(false);
    let ext = if is_fmp4 { "mp4" } else { "ts" };
    let temp_path = format!("{}/{}_temp.{}", output_dir, filename, ext);
    let final_path = format!("{}/{}.mp4", output_dir, filename);

    {
        use std::io::Write;
        let mut file = std::fs::File::create(&temp_path).map_err(|e| e.to_string())?;
        if let Some(ref init) = init_segment { file.write_all(init).map_err(|e| e.to_string())?; }
        for seg in &final_segments { file.write_all(seg).map_err(|e| e.to_string())?; }
    }

    // Remux with ffmpeg if available
    if let Ok(_) = crate::ffmpeg::resolve_ffmpeg_path(app) {
        let cancelled_remux = Arc::new(AtomicBool::new(false));
        match crate::ffmpeg::run_ffmpeg_sync(
            app, job_id, &temp_path, &final_path,
            &crate::ffmpeg::ConversionPreset::Remux, &cancelled_remux,
        ) {
            Ok(path) => { std::fs::remove_file(&temp_path).ok(); return Ok(path); }
            Err(_) => {}
        }
    }

    // Fallback: rename temp as output
    let fallback = format!("{}/{}.{}", output_dir, filename, ext);
    std::fs::rename(&temp_path, &fallback).ok();
    Ok(fallback)
}

/// Download segments with byterange support.
/// For byterange segments, we download with Range headers.
async fn download_byterange_segments(
    app: &tauri::AppHandle,
    job_id: &str,
    client: &Arc<VideoClient>,
    segments: &[parser::HlsSegment],
    cancelled: &AtomicBool,
) -> Result<Vec<Option<Vec<u8>>>, String> {
    let total = segments.len();
    let mut results: Vec<Option<Vec<u8>>> = Vec::with_capacity(total);
    let start_time = std::time::Instant::now();
    let mut total_bytes: u64 = 0;

    for (i, seg) in segments.iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) { break; }

        let data = if let Some(ref br) = seg.byterange {
            // Byterange download with exponential backoff retry
            let mut result = None;
            for attempt in 1..=5u32 {
                match client.get_bytes_range(&seg.url, br.offset.unwrap_or(0), br.length).await {
                    Ok(data) => { result = Some(data); break; }
                    Err(e) => {
                        if attempt < 5 {
                            let delay = std::time::Duration::from_secs(1 << (attempt - 1));
                            tokio::time::sleep(delay).await;
                        } else {
                            log::warn!("Byterange segment {} failed after 5 retries: {}", i, e);
                        }
                    }
                }
            }
            result
        } else {
            // Regular segment with exponential backoff retry
            let mut result = None;
            for attempt in 1..=5u32 {
                match client.get_bytes(&seg.url).await {
                    Ok(data) => { result = Some(data); break; }
                    Err(e) => {
                        if attempt < 5 {
                            let delay = std::time::Duration::from_secs(1 << (attempt - 1));
                            tokio::time::sleep(delay).await;
                        } else {
                            log::warn!("Segment {} failed after 5 retries: {}", i, e);
                        }
                    }
                }
            }
            result
        };

        if let Some(ref d) = data {
            total_bytes += d.len() as u64;
        }

        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 { total_bytes as f64 / elapsed } else { 0.0 };
        let remaining = total - i - 1;
        let avg_time = if i > 0 { elapsed / i as f64 } else { 0.0 };
        let eta = remaining as f64 * avg_time;
        let percent = ((i + 1) as f64 / total as f64) * 100.0;

        let speed_str = if speed > 1_048_576.0 {
            format!("{:.1} MB/s", speed / 1_048_576.0)
        } else {
            format!("{:.0} KB/s", speed / 1024.0)
        };

        let _ = app.emit("download-progress", DownloadProgress {
            job_id: job_id.to_string(),
            percent,
            speed: speed_str,
            eta: format!("{:.0}s", eta),
            status: obfstr!("downloading").to_string(),
            log_line: format!("{}{}/{}", obfstr!("HLS: segment "), i + 1, total),
            file_path: None,
            file_size: Some(total_bytes),
        });

        results.push(data);
    }

    Ok(results)
}

fn is_mp4_box(data: &[u8]) -> bool {
    if data.len() < 8 { return false; }
    let boxes = [b"ftyp", b"styp", b"moof", b"mdat", b"moov"];
    boxes.iter().any(|bt| &data[4..8] == *bt)
}
