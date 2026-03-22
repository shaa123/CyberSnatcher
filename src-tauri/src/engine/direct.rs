use std::sync::atomic::{AtomicBool, Ordering};
use futures_util::StreamExt;
use tauri::Emitter;
use crate::types::DownloadProgress;

pub async fn download_direct(
    app: &tauri::AppHandle,
    job_id: &str,
    url: &str,
    page_url: Option<&str>,
    cookies: Option<&str>,
    output_path: &str,
    cancelled: &AtomicBool,
) -> Result<String, String> {
    let mut client = super::http::VideoClient::new();
    if let Some(page) = page_url { client = client.with_referer(page); }
    else { client = client.with_referer(url); }
    if let Some(c) = cookies { client = client.with_cookies(c); }
    let total_size = client.head_content_length(url).await;

    // Resume support
    let mut start_byte: u64 = 0;
    let existing = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);
    if existing > 0 && total_size.map(|t| existing < t).unwrap_or(false) {
        start_byte = existing;
    }

    let mut req = client.get_streaming(url);
    if start_byte > 0 { req = req.header("Range", format!("bytes={}-", start_byte)); }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    if status != 200 && status != 206 {
        return Err(format!("HTTP {}", resp.status()));
    }

    // If we requested a range but got 200 (server ignored Range header),
    // the response is the full file — reset and overwrite instead of appending.
    if start_byte > 0 && status == 200 {
        start_byte = 0;
    }

    let mut file = if start_byte > 0 {
        std::fs::OpenOptions::new().append(true).open(output_path).map_err(|e| e.to_string())?
    } else {
        std::fs::File::create(output_path).map_err(|e| e.to_string())?
    };

    let mut downloaded = start_byte;
    let start_time = std::time::Instant::now();
    let mut stream = resp.bytes_stream();

    use std::io::Write;
    while let Some(chunk) = stream.next().await {
        if cancelled.load(Ordering::Relaxed) { return Err("Cancelled".into()); }

        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 { (downloaded - start_byte) as f64 / elapsed } else { 0.0 };
        let progress = total_size.map(|t| downloaded as f64 / t as f64 * 100.0).unwrap_or(0.0);
        let eta = if speed > 0.0 { total_size.map(|t| (t - downloaded) as f64 / speed).unwrap_or(0.0) } else { 0.0 };

        let speed_str = if speed > 1_048_576.0 { format!("{:.1} MB/s", speed / 1_048_576.0) }
            else { format!("{:.0} KB/s", speed / 1024.0) };
        let eta_str = if eta > 60.0 { format!("{:.0}:{:02.0}", eta / 60.0, eta % 60.0) }
            else { format!("{:.0}s", eta) };

        let _ = app.emit("download-progress", DownloadProgress {
            job_id: job_id.to_string(),
            percent: progress,
            speed: speed_str,
            eta: eta_str,
            status: "downloading".to_string(),
            log_line: String::new(),
            file_path: None,
            file_size: None,
        });
    }

    let file_size = std::fs::metadata(output_path).map(|m| m.len()).ok();
    let _ = app.emit("download-progress", DownloadProgress {
        job_id: job_id.to_string(), percent: 100.0, speed: String::new(), eta: String::new(),
        status: "complete".to_string(), log_line: "Download complete!".to_string(),
        file_path: Some(output_path.to_string()), file_size: file_size,
    });

    Ok(output_path.to_string())
}
