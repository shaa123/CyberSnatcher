use serde::Serialize;
use std::sync::Arc;

use crate::engine::http::VideoClient;

#[derive(Debug, Serialize, Clone)]
pub struct StreamValidation {
    pub title: String,
    pub duration: Option<f64>,
    pub size: Option<u64>,
    pub qualities: Vec<String>,
}

#[tauri::command]
pub async fn validate_stream(
    manifest_url: String,
    page_url: String,
    stream_type: String,
    min_duration: f64,
    min_file_size: u64,
) -> Result<Option<StreamValidation>, String> {
    let client = Arc::new(
        VideoClient::new()
            .with_referer(&page_url),
    );

    let manifest_text = client.get_text(&manifest_url).await?;

    let (duration, estimated_size, qualities) = match stream_type.as_str() {
        "hls" => estimate_hls(&manifest_text, &manifest_url, &client).await,
        "dash" => estimate_dash(&manifest_text, &manifest_url, &client).await,
        _ => return Err(format!("Unknown stream type: {}", stream_type)),
    }?;

    // Apply filters: duration >= min_duration AND size >= min_file_size
    if let Some(dur) = duration {
        if dur < min_duration {
            log::info!(
                "Stream rejected: duration {:.0}s < {:.0}s minimum",
                dur, min_duration
            );
            return Ok(None);
        }
    }

    if let Some(size) = estimated_size {
        if size < min_file_size {
            log::info!(
                "Stream rejected: size {} < {} minimum",
                size, min_file_size
            );
            return Ok(None);
        }
    }

    // Both must pass (if we can estimate them)
    // If we can't estimate either, reject to be safe
    if duration.is_none() && estimated_size.is_none() {
        log::info!("Stream rejected: could not estimate duration or size");
        return Ok(None);
    }

    let title = page_url
        .replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or("Unknown")
        .to_string();

    Ok(Some(StreamValidation {
        title,
        duration,
        size: estimated_size,
        qualities,
    }))
}

async fn estimate_hls(
    manifest_text: &str,
    manifest_url: &str,
    client: &Arc<VideoClient>,
) -> Result<(Option<f64>, Option<u64>, Vec<String>), String> {
    let parsed = crate::engine::hls::parser::parse_m3u8(manifest_text, manifest_url)?;

    match parsed {
        crate::engine::hls::parser::M3u8Result::Master(master) => {
            let qualities: Vec<String> = master.variants.iter().map(|v| v.label.clone()).collect();

            // Fetch the best variant to estimate duration
            if let Some(best) = master.variants.last() {
                let media_text = client.get_text(&best.url).await?;
                match crate::engine::hls::parser::parse_m3u8(&media_text, &best.url)? {
                    crate::engine::hls::parser::M3u8Result::Media(media) => {
                        let duration = Some(media.total_duration);
                        // Estimate size: sample a few segments and extrapolate
                        let size = estimate_size_from_segments(client, &media.segments.iter().map(|s| s.url.clone()).collect::<Vec<_>>()).await;
                        Ok((duration, size, qualities))
                    }
                    _ => Ok((None, None, qualities)),
                }
            } else {
                Ok((None, None, qualities))
            }
        }
        crate::engine::hls::parser::M3u8Result::Media(media) => {
            let duration = Some(media.total_duration);
            let size = estimate_size_from_segments(client, &media.segments.iter().map(|s| s.url.clone()).collect::<Vec<_>>()).await;
            Ok((duration, size, vec![]))
        }
    }
}

async fn estimate_dash(
    manifest_text: &str,
    manifest_url: &str,
    _client: &Arc<VideoClient>,
) -> Result<(Option<f64>, Option<u64>, Vec<String>), String> {
    let parsed = crate::engine::dash::parser::parse_mpd(manifest_text, manifest_url)?;

    let duration = if parsed.duration > 0.0 {
        Some(parsed.duration)
    } else {
        None
    };

    let qualities: Vec<String> = parsed
        .video_tracks
        .iter()
        .map(|t| t.label.clone())
        .collect();

    // Rough size estimate from bandwidth and duration
    let estimated_size = if let Some(dur) = duration {
        if let Some(best_video) = parsed.video_tracks.last() {
            let video_bytes = (best_video.bandwidth as f64 * dur) / 8.0;
            let audio_bytes = parsed
                .audio_tracks
                .last()
                .map(|a| (a.bandwidth as f64 * dur) / 8.0)
                .unwrap_or(0.0);
            Some((video_bytes + audio_bytes) as u64)
        } else {
            None
        }
    } else {
        None
    };

    Ok((duration, estimated_size, qualities))
}

/// Estimate total size by HEAD-requesting a sample of segments
async fn estimate_size_from_segments(
    client: &Arc<VideoClient>,
    segment_urls: &[String],
) -> Option<u64> {
    if segment_urls.is_empty() {
        return None;
    }

    // Sample up to 3 segments spread across the list
    let sample_count = 3.min(segment_urls.len());
    let step = segment_urls.len() / sample_count;
    let mut total_sample_size: u64 = 0;
    let mut sample_ok = 0u64;

    for i in 0..sample_count {
        let idx = i * step;
        if let Some(size) = client.head_content_length(&segment_urls[idx]).await {
            total_sample_size += size;
            sample_ok += 1;
        }
    }

    if sample_ok == 0 {
        return None;
    }

    let avg_segment_size = total_sample_size / sample_ok;
    Some(avg_segment_size * segment_urls.len() as u64)
}
