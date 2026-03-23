use obfstr::obfstr;
use base64::Engine;
use crate::no_window;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Manager, State};

use crate::ffmpeg::resolve_ffmpeg_path;

enum RecordingStatus {
    Idle,
    Recording,
    Stopping,
}

pub struct RecordingState {
    status: Arc<Mutex<RecordingStatus>>,
    capture_region: Arc<Mutex<(i32, i32, u32, u32)>>,
    output_path: Arc<Mutex<Option<String>>>,
}

impl RecordingState {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(RecordingStatus::Idle)),
            capture_region: Arc::new(Mutex::new((0, 0, 400, 300))),
            output_path: Arc::new(Mutex::new(None)),
        }
    }
}

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, RecordingState>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), String> {
    let ix = x as i32;
    let iy = y as i32;
    let iw = w.max(1.0) as u32;
    let ih = h.max(1.0) as u32;

    // Ensure dimensions are even (required by H.264)
    let iw = iw & !1;
    let ih = ih & !1;

    let ffmpeg_bin = resolve_ffmpeg_path(&app)?;

    {
        let mut status = state.status.lock().map_err(|e| e.to_string())?;
        if matches!(*status, RecordingStatus::Recording) {
            return Err(obfstr!("Already recording").into());
        }
        *status = RecordingStatus::Recording;
    }

    {
        let mut region = state.capture_region.lock().map_err(|e| e.to_string())?;
        *region = (ix, iy, iw, ih);
    }

    // Prepare output path
    let download_dir = dirs::download_dir()
        .or_else(|| dirs::home_dir())
        .ok_or_else(|| obfstr!("Cannot find download directory").to_string())?;

    let timestamp = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    };
    let filename = format!("{}{}.mp4", obfstr!("CyberSnatcher_Recording_"), timestamp);
    let out_path = download_dir.join(&filename);
    let out_path_str = out_path.to_string_lossy().to_string();

    {
        let mut op = state.output_path.lock().map_err(|e| e.to_string())?;
        *op = Some(out_path_str.clone());
    }

    let status_clone = Arc::clone(&state.status);
    let region_clone = Arc::clone(&state.capture_region);

    std::thread::spawn(move || {
        // Spawn ffmpeg: read raw RGBA from stdin, encode to MP4
        let mut ffmpeg = match no_window(Command::new(&ffmpeg_bin)
            .args([
                obfstr!("-y"),
                obfstr!("-f"), obfstr!("rawvideo"),
                obfstr!("-pix_fmt"), obfstr!("rgba"),
                obfstr!("-s"), &format!("{}x{}", iw, ih),
                obfstr!("-r"), "30",
                obfstr!("-i"), obfstr!("-"),
                obfstr!("-c:v"), obfstr!("libx264"),
                obfstr!("-preset"), obfstr!("ultrafast"),
                obfstr!("-crf"), "23",
                obfstr!("-pix_fmt"), obfstr!("yuv420p"),
                obfstr!("-movflags"), obfstr!("+faststart"),
                &out_path_str,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null()))
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                log::error!("{}{}", obfstr!("Failed to spawn ffmpeg: "), e);
                let mut status = status_clone.lock().unwrap();
                *status = RecordingStatus::Idle;
                return;
            }
        };

        let mut stdin = ffmpeg.stdin.take().unwrap();
        let target_interval = std::time::Duration::from_millis(33); // ~30 fps

        loop {
            let start = Instant::now();

            {
                let status = status_clone.lock().unwrap();
                if !matches!(*status, RecordingStatus::Recording) {
                    break;
                }
            }

            let (rx, ry, rw, rh) = {
                let region = region_clone.lock().unwrap();
                *region
            };

            match capture_region(rx, ry, rw, rh) {
                Ok(rgba) => {
                    if stdin.write_all(&rgba).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("{}{}", obfstr!("Frame capture failed: "), e);
                }
            }

            let elapsed = start.elapsed();
            if elapsed < target_interval {
                std::thread::sleep(target_interval - elapsed);
            }
        }

        // Close stdin to signal EOF, then wait for ffmpeg to finish
        drop(stdin);
        let _ = ffmpeg.wait();

        let mut status = status_clone.lock().unwrap();
        *status = RecordingStatus::Idle;
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    state: State<'_, RecordingState>,
) -> Result<String, String> {
    {
        let mut status = state.status.lock().map_err(|e| e.to_string())?;
        match *status {
            RecordingStatus::Recording => {
                *status = RecordingStatus::Stopping;
            }
            _ => return Err(obfstr!("Not recording").into()),
        }
    }

    let path = {
        let op = state.output_path.lock().map_err(|e| e.to_string())?;
        op.clone().ok_or(obfstr!("No output path"))?
    };

    Ok(path)
}

#[tauri::command]
pub async fn update_recording_region(
    state: State<'_, RecordingState>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), String> {
    let mut region = state.capture_region.lock().map_err(|e| e.to_string())?;
    *region = (x as i32, y as i32, w.max(1.0) as u32, h.max(1.0) as u32);
    Ok(())
}

#[tauri::command]
pub async fn capture_preview(x: f64, y: f64, w: f64, h: f64) -> Result<String, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("{}", e))?;
    let monitor = monitors.into_iter().next().ok_or(obfstr!("No monitor"))?;
    let image = monitor
        .capture_region(x as u32, y as u32, w.max(1.0) as u32, h.max(1.0) as u32)
        .map_err(|e| format!("{}", e))?;
    let mut png_buf = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut png_buf, image::ImageFormat::Png)
        .map_err(|e| format!("{}", e))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_buf.into_inner());
    Ok(format!("{}{}", obfstr!("data:image/png;base64,"), b64))
}

fn capture_region(x: i32, y: i32, w: u32, h: u32) -> Result<Vec<u8>, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("{}", e))?;
    let monitor = monitors
        .into_iter()
        .next()
        .ok_or_else(|| obfstr!("No monitor found").to_string())?;

    let image = monitor
        .capture_region(x as u32, y as u32, w, h)
        .map_err(|e| format!("{}{}", obfstr!("Capture failed: "), e))?;

    Ok(image.into_raw())
}
