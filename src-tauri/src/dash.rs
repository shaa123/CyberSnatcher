// dash.rs — DASH MPD Parser + Concurrent Segment Downloader
// Uses dash-mpd crate for parsing, custom download with tokio workers,
// downloads video + audio tracks separately, muxes with ffmpeg.

use dash_mpd::MPD;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex as TokioMutex;

use crate::types::DownloadProgress;

const CONCURRENCY: usize = 6;
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 800;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashQuality {
    pub rep_id: String,
    pub bandwidth: u64,
    pub height: Option<u64>,
    pub label: String,
    pub codecs: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashParseResult {
    pub qualities: Vec<DashQuality>,
    pub duration: Option<f64>,
    pub has_audio: bool,
}

#[derive(Debug, Clone)]
struct DashTrack {
    init_url: String,
    segment_urls: Vec<String>,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Parse a DASH MPD URL — returns available video qualities
pub async fn parse_dash_url(url: &str) -> Result<DashParseResult, String> {
    let client = build_client(None, None);
    let text = fetch_text(&client, url).await?;

    let mpd: MPD = dash_mpd::parse(&text)
        .map_err(|e| format!("MPD parse error: {}", e))?;

    let duration = mpd.mediaPresentationDuration
        .map(|d| d.as_secs_f64());

    let mut qualities = Vec::new();
    let mut has_audio = false;

    for period in &mpd.periods {
        for adapt in &period.adaptations {
            let mime = adapt.mimeType.as_deref().unwrap_or("");
            let content_type = adapt.contentType.as_deref().unwrap_or("");
            let is_video = mime.contains("video") || content_type == "video";
            let is_audio = mime.contains("audio") || content_type == "audio";

            if is_audio { has_audio = true; }

            if is_video {
                for rep in &adapt.representations {
                    let rep_id = rep.id.clone().unwrap_or_default();
                    let bandwidth = rep.bandwidth.unwrap_or(0);
                    let height = rep.height;
                    let codecs = rep.codecs.clone();

                    let label = if let Some(h) = height {
                        format!("{}p", h)
                    } else {
                        format!("{}k", bandwidth / 1000)
                    };

                    qualities.push(DashQuality {
                        rep_id,
                        bandwidth,
                        height,
                        label,
                        codecs,
                    });
                }
            }
        }
    }

    qualities.sort_by_key(|q| q.bandwidth);

    Ok(DashParseResult {
        qualities,
        duration,
        has_audio,
    })
}

/// Download a DASH stream — picks best video + audio, downloads, muxes with ffmpeg
pub async fn download_dash(
    app: &AppHandle,
    job_id: &str,
    url: &str,
    output_dir: &str,
    filename: &str,
    rep_id: Option<&str>,
    cookies: Option<&str>,
    page_url: Option<&str>,
    cancelled: &Arc<AtomicBool>,
) -> Result<String, String> {
    let client = build_client(cookies, page_url);

    emit_progress(app, job_id, 0.0, "downloading", "Parsing DASH manifest...");

    let text = fetch_text(&client, url).await?;
    let mpd: MPD = dash_mpd::parse(&text)
        .map_err(|e| format!("MPD parse error: {}", e))?;

    let duration = mpd.mediaPresentationDuration
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    // Find video and audio tracks
    let mut video_track: Option<DashTrack> = None;
    let mut audio_track: Option<DashTrack> = None;

    for period in &mpd.periods {
        for adapt in &period.adaptations {
            let mime = adapt.mimeType.as_deref().unwrap_or("");
            let content_type = adapt.contentType.as_deref().unwrap_or("");
            let is_video = mime.contains("video") || content_type == "video";
            let is_audio = mime.contains("audio") || content_type == "audio";

            if is_video && video_track.is_none() {
                // Pick the target representation or best available
                let target_rep = if let Some(rid) = rep_id {
                    adapt.representations.iter()
                        .find(|r| r.id.as_deref() == Some(rid))
                } else {
                    None
                };
                let rep = target_rep
                    .or_else(|| adapt.representations.iter()
                        .max_by_key(|r| r.bandwidth.unwrap_or(0)))
                    .ok_or("No video representation found")?;

                video_track = Some(resolve_segments(adapt, rep, url, &mpd)?);
            }

            if is_audio && audio_track.is_none() {
                // Pick best audio
                let rep = adapt.representations.iter()
                    .max_by_key(|r| r.bandwidth.unwrap_or(0))
                    .ok_or("No audio representation found")?;

                audio_track = Some(resolve_segments(adapt, rep, url, &mpd)?);
            }
        }
    }

    let video = video_track.ok_or("No video track in MPD")?;

    emit_progress(app, job_id, 3.0, "downloading",
        &format!("Video: init + {} segments", video.segment_urls.len()));

    if let Some(ref audio) = audio_track {
        emit_progress(app, job_id, 4.0, "downloading",
            &format!("Audio: init + {} segments", audio.segment_urls.len()));
    }

    let safe_name = crate::ytdlp::sanitize_filename(filename);
    let video_path = PathBuf::from(output_dir).join(format!("{}_video.mp4", safe_name));
    let audio_path = PathBuf::from(output_dir).join(format!("{}_audio.mp4", safe_name));
    let mp4_path = PathBuf::from(output_dir).join(format!("{}.mp4", safe_name));

    // Download video init segment
    emit_progress(app, job_id, 5.0, "downloading", "Fetching video init segment...");
    let video_init = fetch_bytes(&client, &video.init_url).await?;

    // Download audio init segment
    let audio_init = if let Some(ref audio) = audio_track {
        emit_progress(app, job_id, 6.0, "downloading", "Fetching audio init segment...");
        Some(fetch_bytes(&client, &audio.init_url).await?)
    } else {
        None
    };

    // Combine all segment URLs for concurrent download
    let all_video_count = video.segment_urls.len();
    let all_audio_count = audio_track.as_ref().map(|a| a.segment_urls.len()).unwrap_or(0);
    let total_segs = all_video_count + all_audio_count;

    let mut all_urls: Vec<String> = video.segment_urls.clone();
    if let Some(ref audio) = audio_track {
        all_urls.extend(audio.segment_urls.clone());
    }

    emit_progress(app, job_id, 8.0, "downloading",
        &format!("Downloading {} segments ({} workers)...", total_segs, CONCURRENCY.min(total_segs)));

    // Concurrent download
    let all_urls = Arc::new(all_urls);
    let next_idx = Arc::new(AtomicUsize::new(0));
    let done_count = Arc::new(AtomicUsize::new(0));
    let results: Arc<TokioMutex<Vec<Option<Vec<u8>>>>> =
        Arc::new(TokioMutex::new(vec![None; total_segs]));

    let worker_count = CONCURRENCY.min(total_segs);
    let mut handles = Vec::new();

    for _ in 0..worker_count {
        let client = client.clone();
        let all_urls = all_urls.clone();
        let next_idx = next_idx.clone();
        let done_count = done_count.clone();
        let results = results.clone();
        let cancelled = cancelled.clone();
        let app = app.clone();
        let job_id = job_id.to_string();
        let total = total_segs;

        handles.push(tokio::spawn(async move {
            loop {
                let idx = next_idx.fetch_add(1, Ordering::SeqCst);
                if idx >= total { break; }
                if cancelled.load(Ordering::Relaxed) { break; }

                let url = &all_urls[idx];
                let mut data: Option<Vec<u8>> = None;

                for attempt in 1..=MAX_RETRIES {
                    match fetch_bytes(&client, url).await {
                        Ok(bytes) => { data = Some(bytes); break; }
                        Err(_) if attempt < MAX_RETRIES => {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                RETRY_DELAY_MS * attempt as u64
                            )).await;
                        }
                        Err(_) => {}
                    }
                }

                if let Some(d) = data {
                    let mut res = results.lock().await;
                    res[idx] = Some(d);
                    done_count.fetch_add(1, Ordering::SeqCst);
                }

                let done = done_count.load(Ordering::SeqCst);
                let pct = 8.0 + (done as f64 / total as f64) * 78.0;
                let _ = app.emit("download-progress", DownloadProgress {
                    job_id: job_id.clone(),
                    percent: pct,
                    speed: String::new(),
                    eta: String::new(),
                    status: "downloading".to_string(),
                    log_line: format!("DASH segment {}/{}", done, total),
                    file_path: None,
                    file_size: None,
                });
            }
        }));
    }

    for h in handles { let _ = h.await; }

    if cancelled.load(Ordering::Relaxed) {
        return Err("Cancelled".to_string());
    }

    emit_progress(app, job_id, 88.0, "converting", "Assembling tracks...");

    // Write video file: init + segments
    {
        let mut file = std::fs::File::create(&video_path)
            .map_err(|e| format!("Create video file: {}", e))?;
        std::io::Write::write_all(&mut file, &video_init)
            .map_err(|e| format!("Write video init: {}", e))?;

        let res = results.lock().await;
        for i in 0..all_video_count {
            if let Some(ref data) = res[i] {
                std::io::Write::write_all(&mut file, data)
                    .map_err(|e| format!("Write video seg {}: {}", i, e))?;
            }
        }
    }

    // Write audio file if we have it
    if let (Some(audio_init_data), Some(_)) = (&audio_init, &audio_track) {
        let mut file = std::fs::File::create(&audio_path)
            .map_err(|e| format!("Create audio file: {}", e))?;
        std::io::Write::write_all(&mut file, audio_init_data)
            .map_err(|e| format!("Write audio init: {}", e))?;

        let res = results.lock().await;
        for i in all_video_count..total_segs {
            if let Some(ref data) = res[i] {
                std::io::Write::write_all(&mut file, data)
                    .map_err(|e| format!("Write audio seg {}: {}", i, e))?;
            }
        }
    }

    // Mux video + audio with ffmpeg
    emit_progress(app, job_id, 92.0, "converting", "Muxing video + audio with ffmpeg...");

    let ffmpeg_bin = crate::ffmpeg::resolve_ffmpeg_path(app)?;

    let mux_result = if audio_track.is_some() && audio_path.exists() {
        // Mux video + audio
        tokio::process::Command::new(&ffmpeg_bin)
            .args([
                "-i", &video_path.to_string_lossy(),
                "-i", &audio_path.to_string_lossy(),
                "-c", "copy",
                "-movflags", "+faststart",
                "-y",
                &mp4_path.to_string_lossy(),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .status()
            .await
            .map_err(|e| format!("ffmpeg error: {}", e))?
    } else {
        // Just remux video
        tokio::process::Command::new(&ffmpeg_bin)
            .args([
                "-i", &video_path.to_string_lossy(),
                "-c", "copy",
                "-movflags", "+faststart",
                "-y",
                &mp4_path.to_string_lossy(),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .status()
            .await
            .map_err(|e| format!("ffmpeg error: {}", e))?
    };

    // Cleanup temp files
    let _ = std::fs::remove_file(&video_path);
    let _ = std::fs::remove_file(&audio_path);

    if !mux_result.success() {
        return Err(format!("ffmpeg mux failed: {}", mux_result));
    }

    // Patch duration
    if duration > 0.0 {
        crate::mp4patch::patch_mp4_duration(&mp4_path, duration);
    }

    let mp4_str = mp4_path.to_string_lossy().to_string();
    let mp4_size = std::fs::metadata(&mp4_path).map(|m| m.len()).ok();

    let _ = app.emit("download-progress", DownloadProgress {
        job_id: job_id.to_string(),
        percent: 100.0,
        speed: String::new(),
        eta: String::new(),
        status: "complete".to_string(),
        log_line: "EXTRACTION COMPLETE ✓".to_string(),
        file_path: Some(mp4_str.clone()),
        file_size: mp4_size,
    });

    Ok(mp4_str)
}

// ── Segment URL resolution ───────────────────────────────────────────────────

fn resolve_segments(
    adapt: &dash_mpd::AdaptationSet,
    rep: &dash_mpd::Representation,
    mpd_url: &str,
    mpd: &MPD,
) -> Result<DashTrack, String> {
    let rep_id = rep.id.clone().unwrap_or_default();
    let bandwidth = rep.bandwidth.unwrap_or(0);

    // Check for SegmentTemplate at representation level, then adaptation level
    let seg_tpl = rep.SegmentTemplate.as_ref()
        .or(adapt.SegmentTemplate.as_ref());

    if let Some(tpl) = seg_tpl {
        return resolve_segment_template(tpl, &rep_id, bandwidth, mpd_url, mpd);
    }

    // Check for SegmentList
    let seg_list = rep.SegmentList.as_ref()
        .or(adapt.SegmentList.as_ref());

    if let Some(list) = seg_list {
        return resolve_segment_list(list, mpd_url);
    }

    // BaseURL fallback
    let base = rep.BaseURL.first()
        .map(|b| b.base.clone())
        .unwrap_or_default();
    let resolved = resolve_url(mpd_url, &base);

    Ok(DashTrack {
        init_url: resolved.clone(),
        segment_urls: vec![resolved],
    })
}

fn resolve_segment_template(
    tpl: &dash_mpd::SegmentTemplate,
    rep_id: &str,
    bandwidth: u64,
    mpd_url: &str,
    mpd: &MPD,
) -> Result<DashTrack, String> {
    let media_tpl = tpl.media.as_deref().unwrap_or("");
    let init_tpl = tpl.initialization.as_deref().unwrap_or("");
    let timescale = tpl.timescale.unwrap_or(1);
    let seg_duration = tpl.duration.unwrap_or(0.0);
    let start_number = tpl.startNumber.unwrap_or(1);

    let init_url = resolve_url(mpd_url, &build_seg_url(init_tpl, rep_id, bandwidth, 0, 0));

    let mut segment_urls = Vec::new();

    // Check for SegmentTimeline
    if let Some(ref timeline) = tpl.SegmentTimeline {
        let mut time: u64 = 0;
        let mut num = start_number;

        for s in &timeline.segments {
            if let Some(t) = s.t {
                time = t;
            }
            let d = s.d;
            let r = s.r.unwrap_or(0);

            for _ in 0..=r {
                let url = build_seg_url(media_tpl, rep_id, bandwidth, num, time);
                segment_urls.push(resolve_url(mpd_url, &url));
                time += d;
                num += 1;
            }
        }
    } else if seg_duration > 0.0 {
        // Calculate total segments from MPD duration
        let total_secs = mpd.mediaPresentationDuration
            .map(|d| d.as_secs_f64())
            .unwrap_or(7200.0);
        let seg_count = (total_secs / (seg_duration as f64 / timescale as f64)).ceil() as u64;

        for i in 0..seg_count {
            let num = start_number + i;
            let time = (i as f64 * seg_duration) as u64;
            let url = build_seg_url(media_tpl, rep_id, bandwidth, num, time);
            segment_urls.push(resolve_url(mpd_url, &url));
        }
    }

    Ok(DashTrack {
        init_url,
        segment_urls,
    })
}

fn resolve_segment_list(
    list: &dash_mpd::SegmentList,
    mpd_url: &str,
) -> Result<DashTrack, String> {
    let init_url = list.Initialization.as_ref()
        .and_then(|i| i.sourceURL.as_ref())
        .map(|u| resolve_url(mpd_url, u))
        .unwrap_or_default();

    let segment_urls: Vec<String> = list.segment_urls.iter()
        .filter_map(|s| s.media.as_ref())
        .map(|u| resolve_url(mpd_url, u))
        .collect();

    Ok(DashTrack {
        init_url,
        segment_urls,
    })
}

fn build_seg_url(tpl: &str, rep_id: &str, bandwidth: u64, num: u64, time: u64) -> String {
    let mut result = tpl.to_string();
    result = result.replace("$RepresentationID$", rep_id);
    result = result.replace("$Bandwidth$", &bandwidth.to_string());
    result = result.replace("$Time$", &time.to_string());

    // Handle $Number$ and $Number%05d$ style
    if result.contains("$Number") {
        if let Some(start) = result.find("$Number%") {
            if let Some(end) = result[start + 8..].find("d$") {
                let width_str = &result[start + 8..start + 8 + end];
                if let Ok(width) = width_str.parse::<usize>() {
                    let formatted = format!("{:0>width$}", num, width = width);
                    let pattern = format!("$Number%{}d$", width_str);
                    result = result.replace(&pattern, &formatted);
                }
            }
        }
        result = result.replace("$Number$", &num.to_string());
    }

    result
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn build_client(cookies: Option<&str>, page_url: Option<&str>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");

    let mut headers = reqwest::header::HeaderMap::new();

    if let Some(cookie_str) = cookies {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(cookie_str) {
            headers.insert(reqwest::header::COOKIE, val);
        }
    }

    if let Some(purl) = page_url {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(purl) {
            headers.insert(reqwest::header::REFERER, val);
        }
        if let Ok(parsed) = url::Url::parse(purl) {
            let origin = format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""));
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&origin) {
                headers.insert(reqwest::header::ORIGIN, val);
            }
        }
    }

    if !headers.is_empty() {
        builder = builder.default_headers(headers);
    }

    builder.build().unwrap_or_default()
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let resp = client.get(url).send().await
        .map_err(|e| format!("Fetch failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Read failed: {}", e))
}

async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client.get(url).send().await
        .map_err(|e| format!("Fetch failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.bytes().await.map(|b| b.to_vec())
        .map_err(|e| format!("Read failed: {}", e))
}

fn resolve_url(base: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }
    match url::Url::parse(base) {
        Ok(base_url) => base_url.join(relative)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| relative.to_string()),
        Err(_) => relative.to_string(),
    }
}

fn emit_progress(app: &AppHandle, job_id: &str, percent: f64, status: &str, log_line: &str) {
    let _ = app.emit("download-progress", DownloadProgress {
        job_id: job_id.to_string(),
        percent,
        speed: String::new(),
        eta: String::new(),
        status: status.to_string(),
        log_line: log_line.to_string(),
        file_path: None,
        file_size: None,
    });
}
