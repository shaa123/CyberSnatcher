// commands/stream.rs — HLS/DASH stream download commands

use crate::hls;
use crate::dash;
use crate::types::{DownloadHandle, DownloadManager};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn parse_hls(url: String) -> Result<hls::HlsParseResult, String> {
    hls::parse_hls_url(&url).await
}

#[tauri::command]
pub async fn download_hls_stream(
    app: AppHandle,
    job_id: String,
    url: String,
    output_dir: String,
    filename: String,
    quality_idx: Option<usize>,
    cookies: Option<String>,
    page_url: Option<String>,
) -> Result<String, String> {
    let cancelled = Arc::new(AtomicBool::new(false));
    {
        let dm = app.state::<DownloadManager>();
        let mut handles = dm.handles.lock().unwrap();
        handles.insert(job_id.clone(), DownloadHandle {
            pid: None,
            cancelled: cancelled.clone(),
        });
    }

    let result = hls::download_hls(
        &app, &job_id, &url, &output_dir, &filename,
        quality_idx, cookies.as_deref(), page_url.as_deref(), &cancelled,
    ).await;

    {
        let dm = app.state::<DownloadManager>();
        if let Ok(mut handles) = dm.handles.lock() {
            handles.remove(&job_id);
        };
    }
    result
}

#[tauri::command]
pub async fn download_hls_live_stream(
    app: AppHandle,
    job_id: String,
    url: String,
    output_dir: String,
    filename: String,
    quality_idx: Option<usize>,
    cookies: Option<String>,
    page_url: Option<String>,
) -> Result<String, String> {
    let cancelled = Arc::new(AtomicBool::new(false));
    {
        let dm = app.state::<DownloadManager>();
        let mut handles = dm.handles.lock().unwrap();
        handles.insert(job_id.clone(), DownloadHandle {
            pid: None,
            cancelled: cancelled.clone(),
        });
    }

    let result = hls::download_hls_live(
        &app, &job_id, &url, &output_dir, &filename,
        quality_idx, cookies.as_deref(), page_url.as_deref(), &cancelled,
    ).await;

    {
        let dm = app.state::<DownloadManager>();
        if let Ok(mut handles) = dm.handles.lock() {
            handles.remove(&job_id);
        };
    }
    result
}

#[tauri::command]
pub async fn parse_dash(url: String) -> Result<dash::DashParseResult, String> {
    dash::parse_dash_url(&url).await
}

#[tauri::command]
pub async fn download_dash_stream(
    app: AppHandle,
    job_id: String,
    url: String,
    output_dir: String,
    filename: String,
    rep_id: Option<String>,
    cookies: Option<String>,
    page_url: Option<String>,
) -> Result<String, String> {
    let cancelled = Arc::new(AtomicBool::new(false));
    {
        let dm = app.state::<DownloadManager>();
        let mut handles = dm.handles.lock().unwrap();
        handles.insert(job_id.clone(), DownloadHandle {
            pid: None,
            cancelled: cancelled.clone(),
        });
    }

    let result = dash::download_dash(
        &app, &job_id, &url, &output_dir, &filename,
        rep_id.as_deref(), cookies.as_deref(), page_url.as_deref(), &cancelled,
    ).await;

    {
        let dm = app.state::<DownloadManager>();
        if let Ok(mut handles) = dm.handles.lock() {
            handles.remove(&job_id);
        };
    }
    result
}
