use obfstr::obfstr;
use crate::types::AppSettings;

#[tauri::command]
pub async fn get_settings() -> Result<AppSettings, String> {
    Ok(AppSettings::default())
}

#[tauri::command]
pub async fn set_download_folder(_path: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn create_folder(path: String) -> Result<(), String> {
    std::fs::create_dir_all(&path).map_err(|e| format!("{}{}", obfstr!("Failed to create folder: "), e))
}

#[tauri::command]
pub async fn delete_folder(path: String) -> Result<(), String> {
    if std::path::Path::new(&path).exists() {
        std::fs::remove_dir_all(&path).map_err(|e| format!("{}{}", obfstr!("Failed to delete folder: "), e))
    } else {
        Err(obfstr!("Folder does not exist").to_string())
    }
}

#[tauri::command]
pub async fn list_folder_contents(path: String) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(&path).map_err(|e| format!("{}{}", obfstr!("Failed to read folder: "), e))?;
    let mut result = vec![];
    for entry in entries {
        if let Ok(entry) = entry {
            if entry.path().is_dir() {
                result.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    result.sort();
    Ok(result)
}

#[tauri::command]
pub async fn open_in_explorer(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    { let _ = std::process::Command::new(obfstr!("explorer")).arg(&path).spawn(); }
    #[cfg(target_os = "macos")]
    { let _ = std::process::Command::new(obfstr!("open")).arg(&path).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = std::process::Command::new(obfstr!("xdg-open")).arg(&path).spawn(); }
    Ok(())
}

#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    { let _ = std::process::Command::new(obfstr!("cmd")).args(["/c", "start", "", &path]).spawn(); }
    #[cfg(target_os = "macos")]
    { let _ = std::process::Command::new(obfstr!("open")).arg(&path).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = std::process::Command::new(obfstr!("xdg-open")).arg(&path).spawn(); }
    Ok(())
}

/// Open the containing folder and highlight the file
#[tauri::command]
pub async fn show_in_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new(obfstr!("explorer"))
            .args(["/select,", &path])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new(obfstr!("open"))
            .args(["-R", &path])
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::process::Command::new(obfstr!("xdg-open"))
                .arg(parent)
                .spawn();
        }
    }
    Ok(())
}
