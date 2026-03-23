use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FavoriteItem {
    pub id: String,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
}

fn favorites_path() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.cybersnatcher.app");
    std::fs::create_dir_all(&dir).ok();
    dir.join("favorites.json")
}

fn load_all() -> Vec<FavoriteItem> {
    let path = favorites_path();
    if !path.exists() {
        return vec![];
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => vec![],
    }
}

fn save_all(items: &[FavoriteItem]) -> Result<(), String> {
    let path = favorites_path();
    let json = serde_json::to_string_pretty(items).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_favorite(
    id: String,
    url: String,
    title: String,
    favicon: Option<String>,
) -> Result<(), String> {
    let mut items = load_all();
    if items.iter().any(|f| f.url == url) {
        return Ok(()); // already exists
    }
    items.insert(
        0,
        FavoriteItem {
            id,
            url,
            title,
            favicon,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        },
    );
    save_all(&items)
}

#[tauri::command]
pub async fn remove_favorite(id: String) -> Result<(), String> {
    let mut items = load_all();
    items.retain(|f| f.id != id);
    save_all(&items)
}

#[tauri::command]
pub async fn list_favorites() -> Result<Vec<FavoriteItem>, String> {
    Ok(load_all())
}

#[tauri::command]
pub async fn is_favorite(url: String) -> Result<bool, String> {
    Ok(load_all().iter().any(|f| f.url == url))
}
