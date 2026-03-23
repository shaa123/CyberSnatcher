use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use crate::types::DownloadProgress;

// ── Binary resolver ──────────────────────────────────────────────────────────

pub fn resolve_ffmpeg_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    // 1. Bundled sidecar
    if let Ok(resource_dir) = app.path().resource_dir() {
        let sidecar = resource_dir.join("binaries").join(ffmpeg_binary_name());
        if sidecar.exists() { return Ok(sidecar); }
    }

    // 2. Next to exe
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(ffmpeg_binary_name());
            if p.exists() { return Ok(p); }
        }
    }

    // 3. Dev mode binaries/
    let dev = std::path::PathBuf::from("binaries").join(ffmpeg_binary_name());
    if dev.exists() { return Ok(dev); }

    // 4. System PATH
    let cmd = if cfg!(windows) { "where" } else { "which" };
    let bin = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    if let Ok(output) = Command::new(cmd).arg(bin).output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(std::path::PathBuf::from(path.lines().next().unwrap_or(&path)));
            }
        }
    }

    Err("ffmpeg not found. Install it or place the binary in the binaries/ folder.".to_string())
}

pub fn check_ffmpeg_available(app: &AppHandle) -> bool {
    resolve_ffmpeg_path(app).is_ok()
}

fn ffmpeg_binary_name() -> &'static str {
    if cfg!(target_os = "windows") { "ffmpeg-x86_64-pc-windows-msvc.exe" }
    else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") { "ffmpeg-aarch64-apple-darwin" }
        else { "ffmpeg-x86_64-apple-darwin" }
    } else { "ffmpeg-x86_64-unknown-linux-gnu" }
}

// ── Conversion presets ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConversionPreset {
    ToMp4,
    ToMp4H264,
    ToMp4H265,
    ToMkv,
    ToWebm,
    ToMp3 { bitrate: u32 },
    ToM4a { bitrate: u32 },
    ToFlac,
    ToWav,
    Remux,
    Compress720p,
    Compress480p,
}

impl ConversionPreset {
    pub fn to_ffmpeg_args(&self, input: &str, output: &str) -> Vec<String> {
        let mut args = vec![
            "-i".to_string(), input.to_string(),
            "-y".to_string(),
        ];

        match self {
            Self::ToMp4 | Self::Remux => {
                args.extend(["-c", "copy", "-movflags", "+faststart"].map(String::from));
            }
            Self::ToMkv => {
                args.extend(["-c", "copy"].map(String::from));
            }
            Self::ToMp4H264 => {
                args.extend([
                    "-c:v", "libx264", "-preset", "medium", "-crf", "23",
                    "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart",
                ].map(String::from));
            }
            Self::ToMp4H265 => {
                args.extend([
                    "-c:v", "libx265", "-preset", "medium", "-crf", "28",
                    "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart",
                    "-tag:v", "hvc1",
                ].map(String::from));
            }
            Self::ToWebm => {
                args.extend([
                    "-c:v", "libvpx-vp9", "-crf", "30", "-b:v", "0",
                    "-c:a", "libopus", "-b:a", "192k",
                ].map(String::from));
            }
            Self::ToMp3 { bitrate } => {
                args.extend(["-vn", "-c:a", "libmp3lame", "-b:a"].map(String::from));
                args.push(format!("{}k", bitrate));
            }
            Self::ToM4a { bitrate } => {
                args.extend(["-vn", "-c:a", "aac", "-b:a"].map(String::from));
                args.push(format!("{}k", bitrate));
            }
            Self::ToFlac => {
                args.extend(["-vn", "-c:a", "flac"].map(String::from));
            }
            Self::ToWav => {
                args.extend(["-vn", "-c:a", "pcm_s16le"].map(String::from));
            }
            Self::Compress720p => {
                args.extend([
                    "-c:v", "libx264", "-preset", "medium", "-crf", "23",
                    "-vf", "scale=-2:720", "-c:a", "aac", "-b:a", "128k",
                    "-movflags", "+faststart",
                ].map(String::from));
            }
            Self::Compress480p => {
                args.extend([
                    "-c:v", "libx264", "-preset", "medium", "-crf", "25",
                    "-vf", "scale=-2:480", "-c:a", "aac", "-b:a", "96k",
                    "-movflags", "+faststart",
                ].map(String::from));
            }
        }

        args.push(output.to_string());
        args
    }

    pub fn output_ext(&self) -> &str {
        match self {
            Self::ToMp4 | Self::ToMp4H264 | Self::ToMp4H265
            | Self::Remux | Self::Compress720p | Self::Compress480p => "mp4",
            Self::ToMkv => "mkv",
            Self::ToWebm => "webm",
            Self::ToMp3 { .. } => "mp3",
            Self::ToM4a { .. } => "m4a",
            Self::ToFlac => "flac",
            Self::ToWav => "wav",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::ToMp4 => "Remux to MP4",
            Self::ToMp4H264 => "Convert to MP4 (H.264)",
            Self::ToMp4H265 => "Convert to MP4 (H.265)",
            Self::ToMkv => "Remux to MKV",
            Self::ToWebm => "Convert to WebM",
            Self::ToMp3 { .. } => "Extract MP3",
            Self::ToM4a { .. } => "Extract M4A",
            Self::ToFlac => "Extract FLAC",
            Self::ToWav => "Extract WAV",
            Self::Remux => "Remux (fix file)",
            Self::Compress720p => "Compress to 720p",
            Self::Compress480p => "Compress to 480p",
        }
    }
}

// ── Run ffmpeg with progress ─────────────────────────────────────────────────

pub fn run_ffmpeg_sync(
    app: &AppHandle,
    job_id: &str,
    input_path: &str,
    output_path: &str,
    preset: &ConversionPreset,
    cancelled: &Arc<AtomicBool>,
) -> Result<String, String> {
    let bin = resolve_ffmpeg_path(app)?;

    // Get duration first
    let duration = get_duration_sync(&bin, input_path);

    let base_args = preset.to_ffmpeg_args(input_path, output_path);

    // Prepend -progress pipe:1 -nostats
    let mut full_args: Vec<String> = vec![
        "-progress".to_string(), "pipe:1".to_string(),
        "-nostats".to_string(),
    ];
    full_args.extend(base_args);

    emit_convert_progress(app, job_id, 0.0, "Starting conversion...");

    let mut child = Command::new(&bin)
        .args(&full_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start ffmpeg: {}", e))?;

    // Read stdout for progress
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            if cancelled.load(Ordering::Relaxed) {
                #[cfg(target_os = "windows")]
                { let _ = Command::new("taskkill").args(["/PID", &child.id().to_string(), "/T", "/F"]).output(); }
                #[cfg(not(target_os = "windows"))]
                { let _ = Command::new("kill").args(["-9", &child.id().to_string()]).output(); }
                std::fs::remove_file(output_path).ok();
                return Err("Cancelled".to_string());
            }

            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "out_time_us" => {
                        if let Ok(us) = value.trim().parse::<f64>() {
                            let current = us / 1_000_000.0;
                            if duration > 0.0 {
                                let pct = (current / duration * 100.0).min(100.0);
                                emit_convert_progress(app, job_id, pct, "");
                            }
                        }
                    }
                    "progress" => {
                        if value.trim() == "end" {
                            emit_convert_progress(app, job_id, 100.0, "Conversion complete!");
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    match child.wait() {
        Ok(status) if status.success() => Ok(output_path.to_string()),
        Ok(status) => {
            std::fs::remove_file(output_path).ok();
            Err(format!("ffmpeg exited with code: {}", status))
        }
        Err(e) => {
            std::fs::remove_file(output_path).ok();
            Err(format!("ffmpeg error: {}", e))
        }
    }
}

/// Mux separate video and audio files into a single MP4.
/// Used by DASH engine to combine video+audio tracks.
pub fn run_ffmpeg_mux(
    app: &AppHandle,
    job_id: &str,
    video_path: &str,
    audio_path: &str,
    output_path: &str,
    cancelled: &Arc<AtomicBool>,
) -> Result<String, String> {
    let bin = resolve_ffmpeg_path(app)?;

    let args = vec![
        "-i", video_path,
        "-i", audio_path,
        "-c", "copy",
        "-movflags", "+faststart",
        "-y",
        output_path,
    ];

    emit_convert_progress(app, job_id, 90.0, "Muxing video + audio...");

    let mut child = Command::new(&bin)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start ffmpeg mux: {}", e))?;

    // Simple wait with cancellation check
    loop {
        if cancelled.load(Ordering::Relaxed) {
            #[cfg(target_os = "windows")]
            { let _ = Command::new("taskkill").args(["/PID", &child.id().to_string(), "/T", "/F"]).output(); }
            #[cfg(not(target_os = "windows"))]
            { let _ = Command::new("kill").args(["-9", &child.id().to_string()]).output(); }
            std::fs::remove_file(output_path).ok();
            return Err("Cancelled".to_string());
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    emit_convert_progress(app, job_id, 100.0, "Mux complete");
                    return Ok(output_path.to_string());
                } else {
                    let stderr = child.stderr.take()
                        .map(|s| {
                            let mut buf = String::new();
                            BufReader::new(s).lines().for_each(|l| {
                                if let Ok(l) = l { buf.push_str(&l); buf.push('\n'); }
                            });
                            buf
                        })
                        .unwrap_or_default();
                    std::fs::remove_file(output_path).ok();
                    return Err(format!("ffmpeg mux failed: {}", stderr.chars().take(200).collect::<String>()));
                }
            }
            Ok(None) => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                std::fs::remove_file(output_path).ok();
                return Err(format!("ffmpeg mux error: {}", e));
            }
        }
    }
}

fn emit_convert_progress(app: &AppHandle, job_id: &str, percent: f64, log_line: &str) {
    let _ = app.emit("download-progress", DownloadProgress {
        job_id: job_id.to_string(),
        percent,
        speed: String::new(),
        eta: String::new(),
        status: "converting".to_string(),
        log_line: log_line.to_string(),
        file_path: None,
        file_size: None,
    });
}

fn get_duration_sync(ffmpeg_bin: &std::path::Path, input: &str) -> f64 {
    // Use ffmpeg -i to get duration from stderr
    let output = Command::new(ffmpeg_bin)
        .args(["-i", input])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();

    if let Ok(output) = output {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(pos) = stderr.find("Duration: ") {
            let dur_str = &stderr[pos + 10..];
            if let Some(comma) = dur_str.find(',') {
                let time_str = &dur_str[..comma]; // "00:10:36.50"
                let parts: Vec<&str> = time_str.split(':').collect();
                if parts.len() == 3 {
                    let h: f64 = parts[0].parse().unwrap_or(0.0);
                    let m: f64 = parts[1].parse().unwrap_or(0.0);
                    let s: f64 = parts[2].parse().unwrap_or(0.0);
                    return h * 3600.0 + m * 60.0 + s;
                }
            }
        }
    }
    0.0
}
