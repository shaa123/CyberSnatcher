use crate::types::{DownloadHandle, DownloadManager, DownloadProgress};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{Emitter, Manager};

/// Start a browser-detected stream download.
/// This ALWAYS uses the native HLS/DASH engines — NEVER yt-dlp.
#[tauri::command]
pub async fn start_browser_download(
    app: tauri::AppHandle,
    job_id: String,
    manifest_url: String,
    stream_type: String,
    page_url: String,
    quality: String,
    output_dir: String,
    filename: String,
) -> Result<String, String> {
    let cancelled = Arc::new(AtomicBool::new(false));

    // Register with DownloadManager for cancellation support
    {
        let dm = app.state::<DownloadManager>();
        let mut handles = dm.handles.lock().unwrap();
        handles.insert(
            job_id.clone(),
            DownloadHandle {
                pid: None,
                cancelled: cancelled.clone(),
            },
        );
    }

    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            job_id: job_id.clone(),
            percent: 0.0,
            speed: "Starting browser download...".to_string(),
            eta: "—".to_string(),
            status: "downloading".to_string(),
            log_line: format!(
                "Browser {}: Starting native download (no yt-dlp)",
                stream_type.to_uppercase()
            ),
            file_path: None,
            file_size: None,
        },
    );

    let result = match stream_type.as_str() {
        "hls" => {
            log::info!(
                "Browser download: routing HLS to native engine (quality={}): {}",
                quality, manifest_url
            );
            crate::engine::hls::download_hls(
                &app,
                &job_id,
                &manifest_url,
                Some(&page_url),
                None, // no cookies from browser detection
                &output_dir,
                &filename,
                &cancelled,
            )
            .await
        }
        "dash" => {
            log::info!(
                "Browser download: routing DASH to native engine (quality={}): {}",
                quality, manifest_url
            );
            crate::engine::dash::download_dash(
                &app,
                &job_id,
                &manifest_url,
                Some(&page_url),
                None,
                &output_dir,
                &filename,
                &cancelled,
            )
            .await
        }
        _ => Err(format!("Unknown stream type: {}", stream_type)),
    };

    // Clean up handle
    {
        let dm = app.state::<DownloadManager>();
        if let Ok(mut handles) = dm.handles.lock() {
            handles.remove(&job_id);
        }
    }

    // Emit completion/error
    match &result {
        Ok(path) => {
            let file_size = std::fs::metadata(path).ok().map(|m| m.len());
            let _ = app.emit(
                "download-progress",
                DownloadProgress {
                    job_id: job_id.clone(),
                    percent: 100.0,
                    speed: String::new(),
                    eta: String::new(),
                    status: "complete".to_string(),
                    log_line: format!("Browser {} download complete", stream_type.to_uppercase()),
                    file_path: Some(path.clone()),
                    file_size,
                },
            );
        }
        Err(e) => {
            let _ = app.emit(
                "download-progress",
                DownloadProgress {
                    job_id: job_id.clone(),
                    percent: 0.0,
                    speed: String::new(),
                    eta: String::new(),
                    status: "error".to_string(),
                    log_line: format!("Browser download error: {}", e),
                    file_path: None,
                    file_size: None,
                },
            );
        }
    }

    result
}
