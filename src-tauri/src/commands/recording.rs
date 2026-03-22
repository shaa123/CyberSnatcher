use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::State;

/// Holds captured frames during a recording session.
struct RecordingFrame {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

enum RecordingStatus {
    Idle,
    Recording,
    Stopping,
}

pub struct RecordingState {
    status: Arc<Mutex<RecordingStatus>>,
    frames: Arc<Mutex<Vec<RecordingFrame>>>,
    capture_region: Arc<Mutex<(i32, i32, u32, u32)>>, // x, y, w, h in screen coords
}

impl RecordingState {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(RecordingStatus::Idle)),
            frames: Arc::new(Mutex::new(Vec::new())),
            capture_region: Arc::new(Mutex::new((0, 0, 400, 300))),
        }
    }
}

#[tauri::command]
pub async fn start_recording(
    state: State<'_, RecordingState>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), String> {
    // Convert to integer screen coords
    let ix = x as i32;
    let iy = y as i32;
    let iw = w.max(1.0) as u32;
    let ih = h.max(1.0) as u32;

    {
        let mut status = state.status.lock().map_err(|e| e.to_string())?;
        match *status {
            RecordingStatus::Recording => return Err("Already recording".into()),
            _ => {}
        }
        *status = RecordingStatus::Recording;
    }

    {
        let mut region = state.capture_region.lock().map_err(|e| e.to_string())?;
        *region = (ix, iy, iw, ih);
    }

    {
        let mut frames = state.frames.lock().map_err(|e| e.to_string())?;
        frames.clear();
    }

    // Spawn capture thread
    let status_clone = Arc::clone(&state.status);
    let frames_clone = Arc::clone(&state.frames);
    let region_clone = Arc::clone(&state.capture_region);

    std::thread::spawn(move || {
        let target_interval = std::time::Duration::from_millis(33); // ~30 fps

        loop {
            let start = Instant::now();

            // Check if we should stop
            {
                let status = status_clone.lock().unwrap();
                match *status {
                    RecordingStatus::Recording => {}
                    _ => break,
                }
            }

            // Get current region
            let (rx, ry, rw, rh) = {
                let region = region_clone.lock().unwrap();
                *region
            };

            // Capture frame
            match capture_region(rx, ry, rw, rh) {
                Ok(frame) => {
                    let mut frames = frames_clone.lock().unwrap();
                    frames.push(frame);
                }
                Err(e) => {
                    log::warn!("Frame capture failed: {}", e);
                }
            }

            // Sleep to maintain target fps
            let elapsed = start.elapsed();
            if elapsed < target_interval {
                std::thread::sleep(target_interval - elapsed);
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    state: State<'_, RecordingState>,
) -> Result<String, String> {
    // Signal stop
    {
        let mut status = state.status.lock().map_err(|e| e.to_string())?;
        match *status {
            RecordingStatus::Recording => {
                *status = RecordingStatus::Stopping;
            }
            _ => return Err("Not recording".into()),
        }
    }

    // Give the capture thread a moment to finish
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Take all frames
    let frames = {
        let mut frames = state.frames.lock().map_err(|e| e.to_string())?;
        std::mem::take(&mut *frames)
    };

    // Reset status
    {
        let mut status = state.status.lock().map_err(|e| e.to_string())?;
        *status = RecordingStatus::Idle;
    }

    if frames.is_empty() {
        return Err("No frames captured".into());
    }

    // Encode to GIF in a blocking thread
    let output_path = encode_gif(frames)?;

    Ok(output_path)
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

fn capture_region(x: i32, y: i32, w: u32, h: u32) -> Result<RecordingFrame, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("Failed to list monitors: {}", e))?;
    let monitor = monitors
        .into_iter()
        .next()
        .ok_or_else(|| "No monitor found".to_string())?;

    let image = monitor
        .capture_region(x, y, w, h)
        .map_err(|e| format!("Capture failed: {}", e))?;

    Ok(RecordingFrame {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}

fn encode_gif(frames: Vec<RecordingFrame>) -> Result<String, String> {
    if frames.is_empty() {
        return Err("No frames to encode".into());
    }

    // Determine output path
    let download_dir = dirs::download_dir()
        .or_else(|| dirs::home_dir())
        .ok_or_else(|| "Cannot find download directory".to_string())?;

    let timestamp = chrono_timestamp();
    let filename = format!("CyberSnatcher_Recording_{}.gif", timestamp);
    let output_path = download_dir.join(&filename);

    let first = &frames[0];
    let width = first.width as u16;
    let height = first.height as u16;

    let file = std::fs::File::create(&output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;

    let mut encoder = gif::Encoder::new(file, width, height, &[])
        .map_err(|e| format!("Failed to create GIF encoder: {}", e))?;

    encoder
        .set_repeat(gif::Repeat::Infinite)
        .map_err(|e| format!("Failed to set repeat: {}", e))?;

    for f in &frames {
        // Convert RGBA to RGB
        let mut rgb: Vec<u8> = Vec::with_capacity((f.width * f.height * 3) as usize);
        for pixel in f.rgba.chunks(4) {
            rgb.push(pixel[0]); // R
            rgb.push(pixel[1]); // G
            rgb.push(pixel[2]); // B
        }

        let mut frame = gif::Frame::from_rgb_speed(
            f.width as u16,
            f.height as u16,
            &mut rgb,
            10, // quality/speed balance
        );
        frame.delay = 3; // 30ms ≈ 30 fps

        encoder
            .write_frame(&frame)
            .map_err(|e| format!("Failed to write frame: {}", e))?;
    }

    Ok(output_path.to_string_lossy().into_owned())
}

fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", dur.as_secs())
}
