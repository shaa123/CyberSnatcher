use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

use super::super::http::VideoClient;
use super::parser::{self, HlsMediaPlaylist};

pub async fn record_live(
    app: &tauri::AppHandle,
    job_id: &str,
    manifest_url: &str,
    client: &Arc<VideoClient>,
    initial_playlist: &HlsMediaPlaylist,
    output_dir: &str,
    filename: &str,
    cancelled: &AtomicBool,
) -> Result<String, String> {
    let mut seen: HashSet<u64> = HashSet::new();
    let mut all_segments: Vec<Vec<u8>> = vec![];
    let mut init_segment: Option<Vec<u8>> = None;
    let mut total_bytes: u64 = 0;
    let start = std::time::Instant::now();

    let decryptor = if let Some(ref enc) = initial_playlist.encryption {
        if enc.method == "AES-128" {
            Some(super::crypto::HlsDecryptor::new(client, &enc.key_uri, enc.iv.clone()).await?)
        } else { None }
    } else { None };

    if let Some(ref init_url) = initial_playlist.init_map_url {
        init_segment = Some(client.get_bytes(init_url).await?);
    }

    while !cancelled.load(Ordering::Relaxed) {
        let manifest_text = match client.get_text(manifest_url).await {
            Ok(t) => t,
            Err(_) => { tokio::time::sleep(std::time::Duration::from_secs(3)).await; continue; }
        };

        let playlist = match parser::parse_m3u8(&manifest_text, manifest_url) {
            Ok(parser::M3u8Result::Media(m)) => m,
            _ => { tokio::time::sleep(std::time::Duration::from_secs(2)).await; continue; }
        };

        if init_segment.is_none() {
            if let Some(ref init_url) = playlist.init_map_url {
                init_segment = client.get_bytes(init_url).await.ok();
            }
        }

        let new_segs: Vec<_> = playlist.segments.iter().filter(|s| !seen.contains(&s.sequence)).cloned().collect();

        for seg in &new_segs {
            if cancelled.load(Ordering::Relaxed) { break; }
            seen.insert(seg.sequence);

            match client.get_bytes(&seg.url).await {
                Ok(mut data) => {
                    if let Some(ref dec) = decryptor {
                        data = dec.decrypt(&data, seg.iv.as_deref(), seg.sequence)?;
                    }
                    total_bytes += data.len() as u64;
                    all_segments.push(data);

                    let _ = app.emit(&format!("download-progress-{}", job_id), serde_json::json!({
                        "status": "recording",
                        "segments": all_segments.len(),
                        "bytes": total_bytes,
                        "elapsed_seconds": start.elapsed().as_secs(),
                    }));
                }
                Err(e) => log::warn!("Live segment {} failed: {}", seg.sequence, e),
            }
        }

        if !playlist.is_live { break; }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    if all_segments.is_empty() { return Err("No segments recorded".into()); }

    let is_fmp4 = init_segment.is_some();
    let ext = if is_fmp4 { "mp4" } else { "ts" };
    let temp_path = format!("{}/{}_temp.{}", output_dir, filename, ext);
    let final_path = format!("{}/{}.mp4", output_dir, filename);

    {
        use std::io::Write;
        let mut file = std::fs::File::create(&temp_path).map_err(|e| e.to_string())?;
        if let Some(ref init) = init_segment { file.write_all(init).map_err(|e| e.to_string())?; }
        for seg in &all_segments { file.write_all(seg).map_err(|e| e.to_string())?; }
    }

    // Remux with ffmpeg if available
    if crate::ffmpeg::resolve_ffmpeg_path(app).is_ok() {
        let cancelled_remux = Arc::new(AtomicBool::new(false));
        match crate::ffmpeg::run_ffmpeg_sync(
            app, job_id, &temp_path, &final_path,
            &crate::ffmpeg::ConversionPreset::Remux, &cancelled_remux,
        ) {
            Ok(path) => { std::fs::remove_file(&temp_path).ok(); return Ok(path); }
            Err(_) => {}
        }
    }

    // Fallback: keep the raw file
    let fallback = format!("{}/{}.{}", output_dir, filename, ext);
    std::fs::rename(&temp_path, &fallback).ok();
    Ok(fallback)
}
