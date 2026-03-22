use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MediaType {
    YouTube, HLS, DASH, DirectVideo, DirectAudio, Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UrlAnalysis {
    pub url: String,
    pub site_name: String,
    pub media_type: MediaType,
    pub title: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<String>,
    pub qualities: Vec<QualityOption>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QualityOption {
    pub label: String,
    pub format_id: String,
    pub file_size: Option<u64>,
    pub ext: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadProgress {
    pub job_id: String,
    pub percent: f64,
    pub speed: String,
    pub eta: String,
    pub status: String,
    pub log_line: String,
    pub file_path: Option<String>,
    pub file_size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub download_folder: String,
    pub max_concurrent: u32,
    pub preferred_quality: String,
    pub preferred_format: String,
    pub auto_convert: bool,
    pub custom_folders: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            download_folder: dirs::download_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            max_concurrent: 3,
            preferred_quality: "1080p".to_string(),
            preferred_format: "mp4".to_string(),
            auto_convert: true,
            custom_folders: vec![],
        }
    }
}

pub struct DownloadHandle {
    pub pid: Option<u32>,
    pub cancelled: Arc<AtomicBool>,
}

pub struct DownloadManager {
    pub handles: Mutex<HashMap<String, DownloadHandle>>,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self { handles: Mutex::new(HashMap::new()) }
    }
}

pub fn detect_site(url: &str) -> String {
    if url.contains("youtube.com") || url.contains("youtu.be") { "YouTube".into() }
    else if url.contains("tiktok.com") { "TikTok".into() }
    else if url.contains("twitter.com") || url.contains("x.com") { "Twitter/X".into() }
    else if url.contains("instagram.com") { "Instagram".into() }
    else if url.contains("reddit.com") { "Reddit".into() }
    else if url.contains(".m3u8") { "HLS Stream".into() }
    else if url.contains(".mpd") { "DASH Stream".into() }
    else { "Website".into() }
}

pub fn detect_type(url: &str) -> MediaType {
    if url.contains("youtube.com") || url.contains("youtu.be") { MediaType::YouTube }
    else if url.contains(".m3u8") { MediaType::HLS }
    else if url.contains(".mpd") { MediaType::DASH }
    else if url.ends_with(".mp4") || url.ends_with(".webm") || url.ends_with(".mkv") { MediaType::DirectVideo }
    else if url.ends_with(".mp3") || url.ends_with(".m4a") { MediaType::DirectAudio }
    else { MediaType::Unknown }
}
