use obfstr::obfstr;

pub mod hls;
pub mod dash;
pub mod download;
pub mod direct;
pub mod http;

/// Route a URL to the correct download engine.
/// Returns Ok(file_path) on success, Err("USE_YTDLP") if yt-dlp should handle it.
pub async fn download_url(
    app: &tauri::AppHandle,
    job_id: &str,
    url: &str,
    page_url: Option<&str>,
    cookies: Option<&str>,
    output_dir: &str,
    filename: &str,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<String, String> {
    // Extract the lowercase extension from the URL path (ignores query string and fragment).
    let url_ext: String = url.split('?').next().unwrap_or("")
        .split('#').next().unwrap_or("")
        .rsplit('.')
        .next()
        .map(|e| format!(".{}", e.to_lowercase()))
        .unwrap_or_default();

    // 1. HLS streams
    if url_ext == obfstr!(".m3u8") {
        log::info!("Routing to HLS engine: {}", url);
        return hls::download_hls(app, job_id, url, page_url, cookies, output_dir, filename, cancelled).await;
    }

    // 2. DASH streams — now uses native engine
    if url_ext == obfstr!(".mpd") {
        log::info!("Routing to DASH engine: {}", url);
        return dash::download_dash(app, job_id, url, page_url, cookies, output_dir, filename, cancelled).await;
    }

    // 3. Direct video files
    let video_exts = [
        ".mp4", ".webm", ".mkv", ".avi",
        ".mov", ".flv", ".m4v", ".ts",
    ];
    for ext in &video_exts {
        if url_ext == *ext {
            log::info!("Routing to direct downloader: {}", url);
            let output_path = format!("{}/{}{}", output_dir, filename, ext);
            return direct::download_direct(app, job_id, url, page_url, cookies, &output_path, cancelled).await;
        }
    }

    // 4. Fall back to yt-dlp
    Err(obfstr!("USE_YTDLP").into())
}
