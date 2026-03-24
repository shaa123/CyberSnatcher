use std::path::Path;
use super::types::{ExportFormat, ScrapedItem};

/// Export scraped items to a file in the given format.
pub fn export_items(
    items: &[ScrapedItem],
    output_path: &str,
    format: &ExportFormat,
) -> Result<String, String> {
    match format {
        ExportFormat::Json => export_json(items, output_path),
        ExportFormat::Csv => export_csv(items, output_path),
        ExportFormat::JsonLines => export_jsonlines(items, output_path),
    }
}

fn export_json(items: &[ScrapedItem], output_path: &str) -> Result<String, String> {
    let path = ensure_extension(output_path, "json");
    let json = serde_json::to_string_pretty(items)
        .map_err(|e| format!("JSON serialization error: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write {}: {}", path, e))?;
    Ok(path)
}

fn export_csv(items: &[ScrapedItem], output_path: &str) -> Result<String, String> {
    let path = ensure_extension(output_path, "csv");

    // Collect all unique field names for headers.
    let mut headers: Vec<String> = Vec::new();
    for item in items {
        for key in item.fields.keys() {
            if !headers.contains(key) {
                headers.push(key.clone());
            }
        }
    }
    headers.sort();

    let mut wtr = csv::Writer::from_path(&path)
        .map_err(|e| format!("Failed to create CSV writer: {}", e))?;

    // Write header row.
    wtr.write_record(&headers)
        .map_err(|e| format!("Failed to write CSV header: {}", e))?;

    // Write data rows.
    for item in items {
        let row: Vec<String> = headers
            .iter()
            .map(|h| item.fields.get(h).cloned().unwrap_or_default())
            .collect();
        wtr.write_record(&row)
            .map_err(|e| format!("Failed to write CSV row: {}", e))?;
    }

    wtr.flush().map_err(|e| format!("Failed to flush CSV: {}", e))?;
    Ok(path)
}

fn export_jsonlines(items: &[ScrapedItem], output_path: &str) -> Result<String, String> {
    let path = ensure_extension(output_path, "jsonl");
    let mut lines = String::new();
    for item in items {
        let line = serde_json::to_string(item)
            .map_err(|e| format!("JSON serialization error: {}", e))?;
        lines.push_str(&line);
        lines.push('\n');
    }
    std::fs::write(&path, lines).map_err(|e| format!("Failed to write {}: {}", path, e))?;
    Ok(path)
}

fn ensure_extension(path: &str, ext: &str) -> String {
    let p = Path::new(path);
    if p.extension().is_some() {
        path.to_string()
    } else {
        format!("{}.{}", path, ext)
    }
}
