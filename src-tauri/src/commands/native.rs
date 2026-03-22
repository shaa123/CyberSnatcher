use crate::types::{DownloadHandle, DownloadManager};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::Manager;

#[tauri::command]
pub async fn native_download(
    app: tauri::AppHandle,
    job_id: String,
    url: String,
    page_url: Option<String>,
    cookies: Option<String>,
    output_dir: String,
    filename: String,
) -> Result<String, String> {
    let cancelled = Arc::new(AtomicBool::new(false));

    // Register with DownloadManager so cancel_download can find this job
    {
        let dm = app.state::<DownloadManager>();
        let mut handles = dm.handles.lock().unwrap();
        handles.insert(job_id.clone(), DownloadHandle {
            pid: None,
            cancelled: cancelled.clone(),
        });
    }

    let result = crate::engine::download_url(
        &app, &job_id, &url,
        page_url.as_deref(),
        cookies.as_deref(),
        &output_dir, &filename,
        &cancelled,
    ).await;

    // Clean up the handle regardless of outcome
    {
        let dm = app.state::<DownloadManager>();
        if let Ok(mut handles) = dm.handles.lock() {
            handles.remove(&job_id);
        }
    }

    result
}
