use obfstr::obfstr;

pub mod parser;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

use super::http::VideoClient;
use crate::types::DownloadProgress;

pub async fn download_dash(
    app: &tauri::AppHandle,
    job_id: &str,
    mpd_url: &str,
    page_url: Option<&str>,
    cookies: Option<&str>,
    output_dir: &str,
    filename: &str,
    cancelled: &AtomicBool,
) -> Result<String, String> {
    let mut cb = VideoClient::new();
    if let Some(page) = page_url {
        cb = cb.with_referer(page);
    } else {
        cb = cb.with_referer(mpd_url);
    }
    if let Some(c) = cookies {
        cb = cb.with_cookies(c);
    }
    let client = Arc::new(cb);

    // Fetch and parse MPD
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            job_id: job_id.to_string(),
            percent: 0.0,
            speed: obfstr!("Fetching DASH manifest...").to_string(),
            eta: "\u{2014}".to_string(),
            status: obfstr!("downloading").to_string(),
            log_line: format!("{}{}", obfstr!("DASH: Fetching manifest "), mpd_url),
            file_path: None,
            file_size: None,
        },
    );

    let mpd_text = client.get_text(mpd_url).await?;
    let manifest = parser::parse_mpd(&mpd_text, mpd_url)?;

    if manifest.is_live {
        return Err(obfstr!("Live DASH streams are not yet supported").into());
    }

    // Check for DRM
    let has_drm_video = manifest.video_tracks.iter().any(|t| t.is_drm);
    let has_drm_audio = manifest.audio_tracks.iter().any(|t| t.is_drm);
    if has_drm_video || has_drm_audio {
        return Err(obfstr!("DRM protected content \u{2014} cannot download. This stream uses Widevine/PlayReady/FairPlay encryption.").into());
    }

    if manifest.video_tracks.is_empty() {
        return Err(obfstr!("No video tracks found in DASH manifest").into());
    }

    // Select best video track (highest bandwidth)
    let video_track = manifest.video_tracks.last().unwrap();
    // Select best audio track if available
    let audio_track = manifest.audio_tracks.last();

    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            job_id: job_id.to_string(),
            percent: 0.0,
            speed: String::new(),
            eta: String::new(),
            status: obfstr!("downloading").to_string(),
            log_line: format!(
                "{}{}{}{}{}{}{}",
                obfstr!("DASH: Video="),
                video_track.label,
                obfstr!(" ("),
                format!("{}bps, {} segments)", video_track.bandwidth, video_track.segment_urls.len()),
                obfstr!(", Audio="),
                audio_track
                    .map(|a| format!("{} ({}bps, {} segments)", a.label, a.bandwidth, a.segment_urls.len()))
                    .unwrap_or_else(|| obfstr!("none").to_string()),
                "",
            ),
            file_path: None,
            file_size: None,
        },
    );

    if cancelled.load(Ordering::Relaxed) {
        return Err(obfstr!("Cancelled").into());
    }

    // Download video init segment
    let video_init = if let Some(ref init_url) = video_track.init_url {
        Some(client.get_bytes(init_url).await?)
    } else {
        None
    };

    // Download video segments
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            job_id: job_id.to_string(),
            percent: 2.0,
            speed: obfstr!("Downloading video segments...").to_string(),
            eta: String::new(),
            status: obfstr!("downloading").to_string(),
            log_line: format!(
                "{}{}",
                obfstr!("DASH: Downloading "),
                format!("{} video segments", video_track.segment_urls.len()),
            ),
            file_path: None,
            file_size: None,
        },
    );

    let video_results = super::download::download_segments(
        app,
        job_id,
        &client,
        &video_track.segment_urls,
        8,
        cancelled,
    )
    .await?;

    if cancelled.load(Ordering::Relaxed) {
        return Err(obfstr!("Cancelled").into());
    }

    // Write video to temp file
    let video_temp = format!("{}/{}_video_temp.m4s", output_dir, filename);
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&video_temp).map_err(|e| e.to_string())?;
        if let Some(ref init) = video_init {
            file.write_all(init).map_err(|e| e.to_string())?;
        }
        for seg_data in &video_results {
            if let Some(data) = seg_data {
                file.write_all(data).map_err(|e| e.to_string())?;
            }
        }
    }

    // Download audio if present
    let audio_temp = if let Some(audio) = audio_track {
        if cancelled.load(Ordering::Relaxed) {
            std::fs::remove_file(&video_temp).ok();
            return Err(obfstr!("Cancelled").into());
        }

        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                job_id: job_id.to_string(),
                percent: 50.0,
                speed: obfstr!("Downloading audio segments...").to_string(),
                eta: String::new(),
                status: obfstr!("downloading").to_string(),
                log_line: format!(
                    "{}{}",
                    obfstr!("DASH: Downloading "),
                    format!("{} audio segments", audio.segment_urls.len()),
                ),
                file_path: None,
                file_size: None,
            },
        );

        let audio_init = if let Some(ref init_url) = audio.init_url {
            Some(client.get_bytes(init_url).await?)
        } else {
            None
        };

        let audio_results = super::download::download_segments(
            app,
            job_id,
            &client,
            &audio.segment_urls,
            8,
            cancelled,
        )
        .await?;

        let audio_path = format!("{}/{}_audio_temp.m4s", output_dir, filename);
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&audio_path).map_err(|e| e.to_string())?;
            if let Some(ref init) = audio_init {
                file.write_all(init).map_err(|e| e.to_string())?;
            }
            for seg_data in &audio_results {
                if let Some(data) = seg_data {
                    file.write_all(data).map_err(|e| e.to_string())?;
                }
            }
        }

        Some(audio_path)
    } else {
        None
    };

    if cancelled.load(Ordering::Relaxed) {
        std::fs::remove_file(&video_temp).ok();
        if let Some(ref ap) = audio_temp {
            std::fs::remove_file(ap).ok();
        }
        return Err(obfstr!("Cancelled").into());
    }

    // Mux video + audio with ffmpeg
    let final_path = format!("{}/{}.mp4", output_dir, filename);

    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            job_id: job_id.to_string(),
            percent: 90.0,
            speed: obfstr!("Muxing video and audio...").to_string(),
            eta: String::new(),
            status: obfstr!("converting").to_string(),
            log_line: obfstr!("DASH: Muxing video + audio into MP4").to_string(),
            file_path: None,
            file_size: None,
        },
    );

    if let Some(ref audio_path) = audio_temp {
        // Try ffmpeg mux with separate video + audio
        if let Ok(_ffmpeg) = crate::ffmpeg::resolve_ffmpeg_path(app) {
            let cancelled_mux = Arc::new(AtomicBool::new(false));
            match crate::ffmpeg::run_ffmpeg_mux(
                app,
                job_id,
                &video_temp,
                audio_path,
                &final_path,
                &cancelled_mux,
            ) {
                Ok(path) => {
                    std::fs::remove_file(&video_temp).ok();
                    std::fs::remove_file(audio_path).ok();
                    return Ok(path);
                }
                Err(e) => {
                    log::warn!("{}{}{}", obfstr!("DASH ffmpeg mux failed: "), e, obfstr!(", trying remux video only"));
                }
            }
        }

        // Fallback: remux video only
        if let Ok(_) = crate::ffmpeg::resolve_ffmpeg_path(app) {
            let cancelled_remux = Arc::new(AtomicBool::new(false));
            match crate::ffmpeg::run_ffmpeg_sync(
                app,
                job_id,
                &video_temp,
                &final_path,
                &crate::ffmpeg::ConversionPreset::Remux,
                &cancelled_remux,
            ) {
                Ok(path) => {
                    std::fs::remove_file(&video_temp).ok();
                    std::fs::remove_file(audio_path).ok();
                    return Ok(path);
                }
                Err(_) => {}
            }
        }

        // Final fallback: rename video temp
        std::fs::remove_file(audio_path).ok();
    } else {
        // No audio: just remux the video
        if let Ok(_) = crate::ffmpeg::resolve_ffmpeg_path(app) {
            let cancelled_remux = Arc::new(AtomicBool::new(false));
            match crate::ffmpeg::run_ffmpeg_sync(
                app,
                job_id,
                &video_temp,
                &final_path,
                &crate::ffmpeg::ConversionPreset::Remux,
                &cancelled_remux,
            ) {
                Ok(path) => {
                    std::fs::remove_file(&video_temp).ok();
                    return Ok(path);
                }
                Err(_) => {}
            }
        }
    }

    // Last resort: just rename video temp
    let fallback = format!("{}/{}.m4s", output_dir, filename);
    std::fs::rename(&video_temp, &fallback).ok();
    Ok(fallback)
}
