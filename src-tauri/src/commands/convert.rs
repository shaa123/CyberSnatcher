use obfstr::obfstr;
use crate::ffmpeg::{resolve_ffmpeg_path, run_ffmpeg_sync, ConversionPreset};
use crate::types::DownloadProgress;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub async fn check_ffmpeg(app: AppHandle) -> Result<bool, String> {
    Ok(resolve_ffmpeg_path(&app).is_ok())
}

#[tauri::command]
pub async fn convert_file(
    app: AppHandle,
    job_id: String,
    input_path: String,
    preset: ConversionPreset,
) -> Result<String, String> {
    let app_clone = app.clone();
    let jid = job_id.clone();

    let output_ext = preset.output_ext().to_string();
    let output_path = input_path
        .rsplit_once('.')
        .map(|(base, _)| format!("{}.{}", base, output_ext))
        .unwrap_or_else(|| format!("{}.{}", input_path, output_ext));

    // Avoid overwriting input
    let final_output = if output_path == input_path {
        let base = input_path.rsplit_once('.').map(|(b, _)| b).unwrap_or(&input_path);
        format!("{}_converted.{}", base, output_ext)
    } else {
        output_path
    };

    let fo = final_output.clone();

    // Run in a thread to not block
    let handle = std::thread::spawn(move || {
        let cancelled = Arc::new(AtomicBool::new(false));
        match run_ffmpeg_sync(&app_clone, &jid, &input_path, &fo, &preset, &cancelled) {
            Ok(path) => {
                let size = std::fs::metadata(&path).map(|m| m.len()).ok();
                let _ = app_clone.emit("download-progress", DownloadProgress {
                    job_id: jid,
                    percent: 100.0,
                    speed: String::new(),
                    eta: String::new(),
                    status: obfstr!("complete").to_string(),
                    log_line: format!("{}{}", obfstr!("Conversion complete: "), path),
                    file_path: Some(path.clone()),
                    file_size: size,
                });
                Ok(path)
            }
            Err(e) => {
                let _ = app_clone.emit("download-progress", DownloadProgress {
                    job_id: jid,
                    percent: -1.0,
                    speed: String::new(),
                    eta: String::new(),
                    status: obfstr!("error").to_string(),
                    log_line: format!("{}{}", obfstr!("Conversion failed: "), e),
                    file_path: None,
                    file_size: None,
                });
                Err(e)
            }
        }
    });

    handle.join().map_err(|_| obfstr!("Thread panicked").to_string())?
}

#[tauri::command]
pub async fn get_media_info(
    _app: AppHandle,
    file_path: String,
) -> Result<MediaInfoResult, String> {
    let metadata = std::fs::metadata(&file_path).map_err(|e| e.to_string())?;
    Ok(MediaInfoResult {
        file_size: metadata.len(),
        file_path,
    })
}

#[derive(serde::Serialize)]
pub struct MediaInfoResult {
    pub file_size: u64,
    pub file_path: String,
}
