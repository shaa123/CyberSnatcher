use obfstr::obfstr;
use crate::types::{detect_site, detect_type, QualityOption, UrlAnalysis};
use crate::ytdlp::resolve_ytdlp_path;
use std::process::Command;

#[tauri::command]
pub async fn check_ytdlp(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(resolve_ytdlp_path(&app).is_ok())
}

#[tauri::command]
pub async fn get_ytdlp_version(app: tauri::AppHandle) -> Result<String, String> {
    let bin = resolve_ytdlp_path(&app)?;
    let output = Command::new(&bin)
        .arg(obfstr!("--version"))
        .output()
        .map_err(|e| format!("{}{}", obfstr!("Failed to run yt-dlp: "), e))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub async fn update_ytdlp(app: tauri::AppHandle) -> Result<String, String> {
    let bin = resolve_ytdlp_path(&app)?;
    let output = Command::new(&bin)
        .args([obfstr!("--update")])
        .output()
        .map_err(|e| format!("{}{}", obfstr!("Failed to run yt-dlp --update: "), e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout.trim(), stderr.trim());

    if combined.contains(obfstr!("Updated yt-dlp to")) || combined.contains(obfstr!("Updating to")) {
        Ok(combined)
    } else if combined.contains(obfstr!("up to date")) || combined.contains(obfstr!("is up-to-date")) {
        Ok(obfstr!("Already up to date.").to_string())
    } else if !output.status.success() {
        // --update may fail if installed via package manager, that's okay
        Ok(obfstr!("Update not available (installed via package manager?).").to_string())
    } else {
        Ok(combined)
    }
}

#[tauri::command]
pub async fn analyze_url(app: tauri::AppHandle, url: String) -> Result<UrlAnalysis, String> {
    let bin = resolve_ytdlp_path(&app)?;

    let output = Command::new(&bin)
        .args([obfstr!("--dump-json"), obfstr!("--no-download"), obfstr!("--no-warnings"), obfstr!("--no-playlist"), &url])
        .output()
        .map_err(|e| format!("{}{}", obfstr!("Failed to run yt-dlp: "), e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Friendly error messages
        let err_msg = if stderr.contains(obfstr!("is not a valid URL")) || stderr.contains(obfstr!("Unsupported URL")) {
            obfstr!("This URL isn't supported. Try a direct video link from YouTube, Twitter, TikTok, etc.").to_string()
        } else if stderr.contains(obfstr!("HTTP Error 403")) || stderr.contains(obfstr!("Forbidden")) {
            obfstr!("Access denied \u{2014} the site may require login or have geo-restrictions.").to_string()
        } else if stderr.contains(obfstr!("HTTP Error 404")) || stderr.contains(obfstr!("not found")) {
            obfstr!("Video not found \u{2014} the link may be broken or the video was deleted.").to_string()
        } else if stderr.contains(obfstr!("urlopen error")) || stderr.contains(obfstr!("Connection")) {
            obfstr!("Connection failed \u{2014} check your internet connection.").to_string()
        } else {
            format!("{}{}", obfstr!("yt-dlp error: "), stderr.lines().last().unwrap_or("Unknown error"))
        };
        return Err(err_msg);
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("{}{}", obfstr!("Failed to parse yt-dlp output: "), e))?;

    let title = json["title"].as_str().map(String::from);
    let thumbnail = json["thumbnail"].as_str().map(String::from);
    let extractor = json["extractor"].as_str().unwrap_or("");
    let site_name = if !extractor.is_empty() {
        extractor.to_string()
    } else {
        detect_site(&url)
    };

    let duration = json["duration"].as_f64().map(|d| {
        let mins = (d / 60.0) as u64;
        let secs = (d % 60.0) as u64;
        format!("{}:{:02}", mins, secs)
    });

    let qualities = extract_qualities(&json);

    Ok(UrlAnalysis {
        url: url.clone(),
        site_name,
        media_type: detect_type(&url),
        title,
        thumbnail,
        duration,
        qualities,
    })
}

fn extract_qualities(json: &serde_json::Value) -> Vec<QualityOption> {
    let mut qualities = vec![];

    if let Some(formats) = json["formats"].as_array() {
        let mut seen_heights = std::collections::HashSet::new();
        let mut has_audio = false;

        // Iterate in reverse (yt-dlp lists best last)
        for fmt in formats.iter().rev() {
            let vcodec = fmt["vcodec"].as_str().unwrap_or("none");
            let acodec = fmt["acodec"].as_str().unwrap_or("none");
            let height = fmt["height"].as_u64().unwrap_or(0);
            let ext = fmt["ext"].as_str().unwrap_or("mp4");
            let format_id = fmt["format_id"].as_str().unwrap_or("");
            let filesize = fmt["filesize"].as_u64().or(fmt["filesize_approx"].as_u64());
            let tbr = fmt["tbr"].as_f64().unwrap_or(0.0);

            // Video formats (deduplicate by height)
            if vcodec != "none" && height > 0 {
                let label = format!("{}p", height);
                if seen_heights.insert(height) {
                    qualities.push(QualityOption {
                        label,
                        format_id: format_id.to_string(),
                        file_size: filesize,
                        ext: ext.to_string(),
                    });
                }
            }
            // Audio only (just one)
            else if vcodec == "none" && acodec != "none" && !has_audio && tbr > 0.0 {
                has_audio = true;
                qualities.push(QualityOption {
                    label: format!("{}{:.0}{}", obfstr!("Audio "), tbr, obfstr!("kbps")),
                    format_id: format_id.to_string(),
                    file_size: filesize,
                    ext: ext.to_string(),
                });
            }
        }
    }

    // Sort: highest resolution first
    qualities.sort_by(|a, b| {
        let ah: u64 = a.label.replace("p", "").parse().unwrap_or(0);
        let bh: u64 = b.label.replace("p", "").parse().unwrap_or(0);
        bh.cmp(&ah)
    });

    qualities
}
