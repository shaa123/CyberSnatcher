use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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

    crate::engine::download_url(
        &app, &job_id, &url,
        page_url.as_deref(),
        cookies.as_deref(),
        &output_dir, &filename,
        &cancelled,
    ).await
}
