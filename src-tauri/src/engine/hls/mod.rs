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
        job_id: job_id.to_string(), percent: 0.0, speed: "Fetching manifest...".to_string(),
        eta: "—".to_string(), status: "downloading".to_string(),
        log_line: format!("HLS: Fetching manifest {}", manifest_url),
        file_path: None, file_size: None,
    });

    let manifest_text = client.get_text(manifest_url).await?;
    let parsed = parser::parse_m3u8(&manifest_text, manifest_url)?;

    let media_playlist = match parsed {
        parser::M3u8Result::Master(master) => {
            let best = master.variants.last().ok_or("No variants in master playlist")?;
            let _ = app.emit("download-progress", DownloadProgress {
                job_id: job_id.to_string(), percent: 0.0, speed: String::new(),
                eta: String::new(), status: "downloading".to_string(),
                log_line: format!("HLS: Selected quality {} ({}bps)", best.label, best.bandwidth),
                file_path: None, file_size: None,
            });
            let media_text = client.get_text(&best.url).await?;
            match parser::parse_m3u8(&media_text, &best.url)? {
                parser::M3u8Result::Media(m) => m,
                _ => return Err("Expected media playlist from variant".into()),
            }
        }
        parser::M3u8Result::Media(media) => media,
    };

    if media_playlist.is_live {
        return live::record_live(app, job_id, manifest_url, &client, &media_playlist, output_dir, filename, cancelled).await;
    }

    let _ = app.emit("download-progress", DownloadProgress {
        job_id: job_id.to_string(), percent: 0.0, speed: String::new(),
        eta: String::new(), status: "downloading".to_string(),
        log_line: format!("HLS: {} segments, {:.0}s, encrypted={}", media_playlist.segments.len(), media_playlist.total_duration, media_playlist.encryption.is_some()),
        file_path: None, file_size: None,
    });

    // Decryption setup
    let decryptor = if let Some(ref enc) = media_playlist.encryption {
        if enc.method == "AES-128" {
            Some(crypto::HlsDecryptor::new(&client, &enc.key_uri, enc.iv.clone()).await?)
        } else { return Err(format!("Unsupported encryption: {}", enc.method)); }
    } else { None };

    // Init segment for fMP4
    let init_segment = if let Some(ref init_url) = media_playlist.init_map_url {
        Some(client.get_bytes(init_url).await?)
    } else { None };

    // Download segments
    let seg_urls: Vec<String> = media_playlist.segments.iter().map(|s| s.url.clone()).collect();
    let results = super::download::download_segments(app, job_id, &client, &seg_urls, 8, cancelled).await?;

    if cancelled.load(Ordering::Relaxed) { return Err("Cancelled".into()); }

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

    if final_segments.is_empty() { return Err("All segments failed".into()); }

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

fn is_mp4_box(data: &[u8]) -> bool {
    if data.len() < 8 { return false; }
    let boxes = [b"ftyp", b"styp", b"moof", b"mdat", b"moov"];
    boxes.iter().any(|bt| &data[4..8] == *bt)
}
