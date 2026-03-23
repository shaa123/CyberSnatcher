use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrowserSettings {
    #[serde(rename = "minDuration")]
    pub min_duration: f64,
    #[serde(rename = "minFileSize")]
    pub min_file_size: u64,
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            min_duration: 40.0,
            min_file_size: 2097152, // 2MB
        }
    }
}

fn settings_path() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.cybersnatcher.app");
    std::fs::create_dir_all(&dir).ok();
    dir.join("browser_settings.json")
}

#[tauri::command]
pub async fn save_browser_settings(min_duration: f64, min_file_size: u64) -> Result<(), String> {
    let settings = BrowserSettings {
        min_duration,
        min_file_size,
    };
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(), json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_browser_settings() -> Result<BrowserSettings, String> {
    let path = settings_path();
    if !path.exists() {
        return Ok(BrowserSettings::default());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}
