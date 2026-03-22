use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use tauri::Emitter;
use tokio::sync::Semaphore;

use super::http::VideoClient;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SegmentProgress {
    pub done: u64,
    pub total: u64,
    pub bytes: u64,
    pub speed_bps: f64,
    pub eta_seconds: f64,
}

pub async fn download_segments(
    app: &tauri::AppHandle,
    job_id: &str,
    client: &Arc<VideoClient>,
    urls: &[String],
    concurrency: usize,
    cancelled: &AtomicBool,
) -> Result<Vec<Option<Vec<u8>>>, String> {
    let total = urls.len() as u64;
    let done = Arc::new(AtomicU64::new(0));
    let bytes = Arc::new(AtomicU64::new(0));
    let start_time = std::time::Instant::now();
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let results: Arc<tokio::sync::Mutex<Vec<Option<Vec<u8>>>>> =
        Arc::new(tokio::sync::Mutex::new(vec![None; urls.len()]));

    let mut handles = vec![];

    for (i, url) in urls.iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) { break; }

        let permit = semaphore.clone().acquire_owned().await.map_err(|e| e.to_string())?;
        let client = client.clone();
        let url = url.clone();
        let results = results.clone();
        let done = done.clone();
        let bytes_counter = bytes.clone();
        let app = app.clone();
        let job_id = job_id.to_string();

        let handle = tokio::spawn(async move {
            let _permit = permit;
            let mut data: Option<Vec<u8>> = None;

            for attempt in 1..=3u32 {
                match client.get_bytes(&url).await {
                    Ok(segment_data) => {
                        data = Some(segment_data);
                        break;
                    }
                    Err(e) => {
                        if attempt < 3 {
                            tokio::time::sleep(std::time::Duration::from_millis(800 * attempt as u64)).await;
                        } else {
                            log::warn!("Segment {} failed after 3 retries: {}", i, e);
                        }
                    }
                }
            }

            if let Some(ref d) = data {
                bytes_counter.fetch_add(d.len() as u64, Ordering::Relaxed);
            }
            done.fetch_add(1, Ordering::Relaxed);

            let mut res = results.lock().await;
            res[i] = data;

            let completed = done.load(Ordering::Relaxed);
            let total_bytes = bytes_counter.load(Ordering::Relaxed);
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 { total_bytes as f64 / elapsed } else { 0.0 };
            let remaining = total - completed;
            let avg_time = if completed > 0 { elapsed / completed as f64 } else { 0.0 };
            let eta = remaining as f64 * avg_time;

            let _ = app.emit(&format!("download-segments-{}", job_id), SegmentProgress {
                done: completed, total, bytes: total_bytes, speed_bps: speed, eta_seconds: eta,
            });
        });

        handles.push(handle);
    }

    for h in handles { h.await.ok(); }

    let final_results = results.lock().await.clone();
    Ok(final_results)
}
