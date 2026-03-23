use obfstr::obfstr;
use crate::no_window;
use std::path::PathBuf;
use std::process::Command;
use tauri::Manager;

/// Resolve the path to the yt-dlp binary.
/// Checks: bundled sidecar → system PATH → error
pub fn resolve_ytdlp_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // 1. Check for bundled sidecar binary next to the app
    if let Ok(resource_dir) = app.path().resource_dir() {
        let sidecar = resource_dir.join(obfstr!("binaries")).join(ytdlp_binary_name());
        if sidecar.exists() {
            return Ok(sidecar);
        }
    }

    // 2. Check in the exe directory (for dev builds)
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(dir) = exe_dir.parent() {
            let sidecar = dir.join(ytdlp_binary_name());
            if sidecar.exists() {
                return Ok(sidecar);
            }
        }
    }

    // 3. Check src-tauri/binaries/ (dev mode)
    let dev_path = PathBuf::from(obfstr!("binaries")).join(ytdlp_binary_name());
    if dev_path.exists() {
        return Ok(dev_path);
    }

    // 3b. Also check for a plain "yt-dlp" binary in the binaries/ folders
    let plain_name = if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" };
    if let Ok(resource_dir) = app.path().resource_dir() {
        let plain = resource_dir.join(obfstr!("binaries")).join(plain_name);
        if plain.exists() {
            return Ok(plain);
        }
    }
    let dev_plain = PathBuf::from(obfstr!("binaries")).join(plain_name);
    if dev_plain.exists() {
        return Ok(dev_plain);
    }

    // 4. Fall back to system PATH
    let cmd = if cfg!(windows) { "where" } else { "which" };
    let bin = if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" };
    if let Ok(output) = no_window(Command::new(cmd).arg(bin)).output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(PathBuf::from(path.lines().next().unwrap_or(&path)));
            }
        }
    }

    Err(obfstr!("yt-dlp not found. Install it from https://github.com/yt-dlp/yt-dlp/releases or place it in the binaries/ folder.").to_string())
}

fn ytdlp_binary_name() -> String {
    if cfg!(target_os = "windows") {
        obfstr!("yt-dlp-x86_64-pc-windows-msvc.exe").to_string()
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            obfstr!("yt-dlp-aarch64-apple-darwin").to_string()
        } else {
            obfstr!("yt-dlp-x86_64-apple-darwin").to_string()
        }
    } else {
        obfstr!("yt-dlp-x86_64-unknown-linux-gnu").to_string()
    }
}

/// Sanitize a string for use as a filename
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .chars()
        .take(200)
        .collect()
}
