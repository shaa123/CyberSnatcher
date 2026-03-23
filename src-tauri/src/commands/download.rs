use obfstr::obfstr;
use crate::license::{LicenseState, require_license_for_quality};
use crate::no_window;
use crate::types::{DownloadHandle, DownloadManager, DownloadProgress};
use crate::ytdlp::{resolve_ytdlp_path, sanitize_filename};
use crate::ffmpeg::resolve_ffmpeg_path;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

fn emit_progress(app: &AppHandle, job_id: &str, progress: DownloadProgress) {
    // Emit both per-job and global events
    let _ = app.emit(&format!("download-progress-{}", job_id), &progress);
    let _ = app.emit("download-progress", &progress);
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    job_id: String,
    url: String,
    title: String,
    output_dir: String,
    format_quality: String,
    format_type: String,
    write_subs: Option<bool>,
) -> Result<(), String> {
    // Gate high-quality downloads behind license
    let license = app.state::<LicenseState>();
    require_license_for_quality(&license, &format_quality)?;

    let bin = resolve_ytdlp_path(&app)?;
    let cancelled = Arc::new(AtomicBool::new(false));

    // Register handle before spawning
    {
        let dm = app.state::<DownloadManager>();
        let mut handles = dm.handles.lock().unwrap();
        handles.insert(job_id.clone(), DownloadHandle {
            pid: None,
            cancelled: cancelled.clone(),
        });
    }

    let app_clone = app.clone();
    let jid = job_id.clone();
    let subs = write_subs.unwrap_or(false);

    std::thread::spawn(move || {
        run_download(app_clone, jid, bin.to_string_lossy().to_string(), url, title, output_dir, format_quality, format_type, subs, cancelled);
    });

    Ok(())
}

fn run_download(
    app: AppHandle,
    job_id: String,
    bin_path: String,
    url: String,
    title: String,
    output_dir: String,
    format_quality: String,
    format_type: String,
    write_subs: bool,
    cancelled: Arc<AtomicBool>,
) {
    // Build output template
    let safe_title = sanitize_filename(&title);
    let output_template = if output_dir.is_empty() {
        format!("{}.%(ext)s", safe_title)
    } else {
        let dir = output_dir.replace('\\', "/").trim_end_matches('/').to_string();
        format!("{}/{}.%(ext)s", dir, safe_title)
    };

    let mut args: Vec<String> = vec![
        obfstr!("--newline").to_string(),
        obfstr!("--no-warnings").to_string(),
        obfstr!("--no-playlist").to_string(),
        obfstr!("--progress-template").to_string(),
        obfstr!("download:CYBERPROG|||%(progress._percent_str)s|||%(progress._speed_str)s|||%(progress._eta_str)s").to_string(),
        obfstr!("-o").to_string(),
        output_template.clone(),
    ];

    // Point yt-dlp to our ffmpeg for merging
    if let Ok(ffmpeg_bin) = resolve_ffmpeg_path(&app) {
        if let Some(ffmpeg_dir) = ffmpeg_bin.parent() {
            args.push(obfstr!("--ffmpeg-location").to_string());
            args.push(ffmpeg_dir.to_string_lossy().to_string());
        }
    }

    // Format/quality selection
    if format_quality == "audio" {
        args.push(obfstr!("-x").to_string());
        args.push(obfstr!("--audio-format").to_string());
        args.push(obfstr!("mp3").to_string());
    } else if !format_quality.is_empty() && format_quality != "best" {
        let height = format_quality.replace("p", "");
        args.push(obfstr!("-f").to_string());
        args.push(format!(
            "{}{}{}{}",
            obfstr!("bestvideo[height<="),
            height,
            obfstr!("]+bestaudio/best[height<="),
            format!("{}]", height)
        ));
    }

    // Container format
    if !format_type.is_empty() && format_type != "Default" && format_quality != "audio" {
        args.push(obfstr!("--merge-output-format").to_string());
        args.push(format_type.to_lowercase());
    }

    // Subtitle download
    if write_subs {
        args.push(obfstr!("--write-subs").to_string());
        args.push(obfstr!("--write-auto-subs").to_string());
        args.push(obfstr!("--sub-langs").to_string());
        args.push(obfstr!("all").to_string());
        args.push(obfstr!("--embed-subs").to_string());
    }

    args.push(url.clone());

    // Emit: starting
    emit_progress(&app, &job_id, DownloadProgress {
        job_id: job_id.clone(),
        percent: 0.0,
        speed: obfstr!("Starting...").to_string(),
        eta: "\u{2014}".to_string(),
        status: obfstr!("downloading").to_string(),
        log_line: format!("{}{}", obfstr!("Initiating download: "), url),
        file_path: None,
        file_size: None,
    });

    let child = no_window(Command::new(&bin_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()))
        .spawn();

    match child {
        Ok(mut process) => {
            let pid = process.id();

            // Store PID for cancellation
            {
                let dm = app.state::<DownloadManager>();
                if let Ok(mut handles) = dm.handles.lock() {
                    if let Some(h) = handles.get_mut(&job_id) {
                        h.pid = Some(pid);
                    }
                };
            }

            // Spawn stderr reader thread
            let stderr = process.stderr.take();
            let app_err = app.clone();
            let jid_err = job_id.clone();
            let stderr_thread = std::thread::spawn(move || {
                if let Some(stderr) = stderr {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().flatten() {
                        if !line.trim().is_empty() {
                            emit_progress(&app_err, &jid_err, DownloadProgress {
                                job_id: jid_err.clone(),
                                percent: -1.0,
                                speed: String::new(),
                                eta: String::new(),
                                status: obfstr!("downloading").to_string(),
                                log_line: format!("{}{}", obfstr!("[stderr] "), line),
                                file_path: None,
                                file_size: None,
                            });
                        }
                    }
                }
            });

            // Read stdout for progress
            if let Some(stdout) = process.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    // Check cancellation
                    if cancelled.load(Ordering::Relaxed) {
                        kill_process(pid);
                        emit_progress(&app, &job_id, DownloadProgress {
                            job_id: job_id.clone(),
                            percent: -1.0,
                            speed: String::new(),
                            eta: String::new(),
                            status: obfstr!("cancelled").to_string(),
                            log_line: obfstr!("Download cancelled by user.").to_string(),
                            file_path: None,
                            file_size: None,
                        });
                        cleanup_handle(&app, &job_id);
                        return;
                    }

                    if line.contains(obfstr!("CYBERPROG|||")) {
                        let parts: Vec<&str> = line.split("|||").collect();
                        if parts.len() >= 4 {
                            let percent_str = parts[1].trim().replace('%', "");
                            let percent: f64 = percent_str.trim().parse().unwrap_or(0.0);
                            let speed = parts[2].trim().to_string();
                            let eta = parts[3].trim().to_string();

                            emit_progress(&app, &job_id, DownloadProgress {
                                job_id: job_id.clone(),
                                percent,
                                speed: speed.clone(),
                                eta: eta.clone(),
                                status: obfstr!("downloading").to_string(),
                                log_line: String::new(),
                                file_path: None,
                                file_size: None,
                            });
                        }
                    } else if line.contains(obfstr!("[Merger]")) || line.contains(obfstr!("[ffmpeg]")) {
                        emit_progress(&app, &job_id, DownloadProgress {
                            job_id: job_id.clone(),
                            percent: 99.0,
                            speed: String::new(),
                            eta: String::new(),
                            status: obfstr!("converting").to_string(),
                            log_line: line.clone(),
                            file_path: None,
                            file_size: None,
                        });
                    } else if !line.trim().is_empty() {
                        emit_progress(&app, &job_id, DownloadProgress {
                            job_id: job_id.clone(),
                            percent: -1.0,
                            speed: String::new(),
                            eta: String::new(),
                            status: obfstr!("downloading").to_string(),
                            log_line: line.clone(),
                            file_path: None,
                            file_size: None,
                        });
                    }
                }
            }

            // Wait for stderr thread
            let _ = stderr_thread.join();

            // Wait for process to finish
            match process.wait() {
                Ok(status) => {
                    if status.success() {
                        // Find the output file
                        let file_path = find_output_file(&output_template);
                        let file_size = std::fs::metadata(&file_path).map(|m| m.len()).ok();

                        emit_progress(&app, &job_id, DownloadProgress {
                            job_id: job_id.clone(),
                            percent: 100.0,
                            speed: String::new(),
                            eta: String::new(),
                            status: obfstr!("complete").to_string(),
                            log_line: obfstr!("EXTRACTION COMPLETE \u{2713}").to_string(),
                            file_path: Some(file_path),
                            file_size,
                        });
                    } else {
                        emit_progress(&app, &job_id, DownloadProgress {
                            job_id: job_id.clone(),
                            percent: -1.0,
                            speed: String::new(),
                            eta: String::new(),
                            status: obfstr!("error").to_string(),
                            log_line: format!("{}{}", obfstr!("yt-dlp exited with code: "), status),
                            file_path: None,
                            file_size: None,
                        });
                    }
                }
                Err(e) => {
                    emit_progress(&app, &job_id, DownloadProgress {
                        job_id: job_id.clone(),
                        percent: -1.0,
                        speed: String::new(),
                        eta: String::new(),
                        status: obfstr!("error").to_string(),
                        log_line: format!("{}{}", obfstr!("Error waiting for process: "), e),
                        file_path: None,
                        file_size: None,
                    });
                }
            }

            cleanup_handle(&app, &job_id);
        }
        Err(e) => {
            emit_progress(&app, &job_id, DownloadProgress {
                job_id: job_id.clone(),
                percent: -1.0,
                speed: String::new(),
                eta: String::new(),
                status: obfstr!("error").to_string(),
                log_line: format!("{}{}", obfstr!("Failed to start yt-dlp: "), format!("{}. {}", e, obfstr!("Is it installed?"))),
                file_path: None,
                file_size: None,
            });
            cleanup_handle(&app, &job_id);
        }
    }
}

fn kill_process(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let _ = no_window(Command::new(obfstr!("taskkill"))
            .args(["/PID", &pid.to_string(), "/T", "/F"]))
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new(obfstr!("kill"))
            .args(["-9", &pid.to_string()])
            .output();
    }
}

fn cleanup_handle(app: &AppHandle, job_id: &str) {
    let dm = app.state::<DownloadManager>();
    if let Ok(mut handles) = dm.handles.lock() {
        handles.remove(job_id);
    };
}

fn find_output_file(template: &str) -> String {
    for ext in &["mp4", "mkv", "webm", "mp3", "m4a", "opus", "flac", "wav"] {
        let path = template.replace("%(ext)s", ext);
        if std::path::Path::new(&path).exists() {
            return path;
        }
    }
    template.replace("%(ext)s", "mp4")
}

#[tauri::command]
pub async fn cancel_download(app: AppHandle, job_id: String) -> Result<(), String> {
    let dm = app.state::<DownloadManager>();
    if let Ok(handles) = dm.handles.lock() {
        if let Some(handle) = handles.get(&job_id) {
            handle.cancelled.store(true, Ordering::Relaxed);
            if let Some(pid) = handle.pid {
                kill_process(pid);
            }
        }
    };
    Ok(())
}
