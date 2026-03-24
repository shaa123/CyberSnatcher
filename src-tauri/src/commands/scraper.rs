use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::scraper::engine;
use crate::scraper::exporter;
use crate::scraper::types::*;

/// Start a scrape job. Progress is emitted via the "scrape-progress" event.
#[tauri::command]
pub async fn start_scrape(
    app: AppHandle,
    job_id: String,
    config: SpiderConfig,
) -> Result<ScrapeResult, String> {
    let (tx, mut rx) = mpsc::unbounded_channel::<ScrapeProgress>();

    // Spawn a task to forward progress events to the frontend.
    let app_clone = app.clone();
    let jid = job_id.clone();
    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            let _ = app_clone.emit("scrape-progress", &progress);
        }
        // Signal completion.
        let _ = app_clone.emit(
            "scrape-progress",
            &ScrapeProgress {
                job_id: jid,
                pages_crawled: 0,
                pages_total: 0,
                items_scraped: 0,
                current_url: String::new(),
                status: "finished".into(),
                log_line: "Job finished".into(),
            },
        );
    });

    let result = engine::run_spider(job_id, config, tx).await;
    Ok(result)
}

/// Export scraped data to a file.
#[tauri::command]
pub async fn export_scrape_data(
    items: Vec<ScrapedItem>,
    output_path: String,
    format: ExportFormat,
) -> Result<String, String> {
    exporter::export_items(&items, &output_path, &format)
}

/// Quick preview: fetch a single URL and extract items using the given rules.
/// Useful for testing selectors before running a full crawl.
#[tauri::command]
pub async fn preview_scrape(
    url: String,
    field_rules: Vec<FieldRule>,
    item_selector: Option<String>,
) -> Result<Vec<ScrapedItem>, String> {
    let client = crate::scraper::middleware::build_client(None, &std::collections::HashMap::new())?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let html = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let items = crate::scraper::selector::extract_items(
        &html,
        &field_rules,
        item_selector.as_deref(),
    );

    Ok(items)
}
