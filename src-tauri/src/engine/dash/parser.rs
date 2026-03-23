use obfstr::obfstr;
use url::Url;

#[derive(Debug, Clone)]
pub struct DashManifest {
    pub duration: f64,
    pub video_tracks: Vec<DashTrack>,
    pub audio_tracks: Vec<DashTrack>,
    pub is_live: bool,
}

#[derive(Debug, Clone)]
pub struct DashTrack {
    pub id: String,
    pub bandwidth: u64,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub mime_type: String,
    pub codecs: Option<String>,
    pub init_url: Option<String>,
    pub segment_urls: Vec<String>,
    pub label: String,
    pub is_drm: bool,
}

/// Parse an MPD manifest into a DashManifest
pub fn parse_mpd(content: &str, base_url: &str) -> Result<DashManifest, String> {
    let doc = roxmltree::Document::parse(content)
        .map_err(|e| format!("{}{}", obfstr!("Failed to parse MPD XML: "), e))?;

    let mpd = doc.root_element();
    if mpd.tag_name().name() != obfstr!("MPD") {
        return Err(obfstr!("Root element is not MPD").into());
    }

    // Parse duration from mediaPresentationDuration (ISO 8601 duration)
    let duration = mpd
        .attribute(obfstr!("mediaPresentationDuration"))
        .map(parse_iso_duration)
        .unwrap_or(0.0);

    let is_live = mpd.attribute(obfstr!("type")).map(|t| t == obfstr!("dynamic")).unwrap_or(false);

    // Get MPD-level BaseURL
    let mpd_base = find_base_url(&mpd, base_url);

    let mut video_tracks = vec![];
    let mut audio_tracks = vec![];

    // Parse all Periods
    for period in mpd.children().filter(|n| n.tag_name().name() == obfstr!("Period")) {
        let period_base = find_base_url(&period, &mpd_base);

        for adaptation_set in period.children().filter(|n| n.tag_name().name() == obfstr!("AdaptationSet")) {
            let as_base = find_base_url(&adaptation_set, &period_base);

            let as_mime = adaptation_set
                .attribute(obfstr!("mimeType"))
                .unwrap_or("")
                .to_string();
            let as_content_type = adaptation_set
                .attribute(obfstr!("contentType"))
                .unwrap_or("")
                .to_string();

            // Check for DRM
            let has_drm = adaptation_set.children().any(|n| {
                n.tag_name().name() == obfstr!("ContentProtection")
                    && n.attribute(obfstr!("schemeIdUri"))
                        .map(|s| {
                            s.contains(obfstr!("edef8ba9-79d6-4ace-a3c8-27dcd51d21ed")) // Widevine
                            || s.contains(obfstr!("9a04f079-9840-4286-ab92-e65be0885f95")) // PlayReady
                            || s.contains(obfstr!("94ce86fb-07ff-4f43-adb8-93d2fa968ca2")) // FairPlay
                        })
                        .unwrap_or(false)
            });

            // Get AdaptationSet-level SegmentTemplate
            let as_segment_template = adaptation_set
                .children()
                .find(|n| n.tag_name().name() == obfstr!("SegmentTemplate"));

            for repr in adaptation_set.children().filter(|n| n.tag_name().name() == obfstr!("Representation")) {
                let repr_base = find_base_url(&repr, &as_base);

                let id = repr.attribute(obfstr!("id")).unwrap_or("0").to_string();
                let bandwidth = repr
                    .attribute(obfstr!("bandwidth"))
                    .and_then(|b| b.parse::<u64>().ok())
                    .unwrap_or(0);
                let height = repr
                    .attribute(obfstr!("height"))
                    .and_then(|h| h.parse::<u32>().ok());
                let width = repr
                    .attribute(obfstr!("width"))
                    .and_then(|w| w.parse::<u32>().ok());
                let mime = repr
                    .attribute(obfstr!("mimeType"))
                    .unwrap_or(&as_mime)
                    .to_string();
                let codecs = repr.attribute(obfstr!("codecs")).map(|c| c.to_string());

                let label = if let Some(h) = height {
                    format!("{}p", h)
                } else {
                    format!("{}k", bandwidth / 1000)
                };

                // Get Representation-level SegmentTemplate (overrides AS-level)
                let repr_segment_template = repr
                    .children()
                    .find(|n| n.tag_name().name() == obfstr!("SegmentTemplate"))
                    .or(as_segment_template);

                let (init_url, segment_urls) = if let Some(seg_tpl) = repr_segment_template {
                    parse_segment_template(&seg_tpl, &repr_base, &id, bandwidth, duration)
                } else if let Some(seg_list) = repr.children().find(|n| n.tag_name().name() == obfstr!("SegmentList"))
                    .or_else(|| adaptation_set.children().find(|n| n.tag_name().name() == obfstr!("SegmentList")))
                {
                    parse_segment_list(&seg_list, &repr_base)
                } else if let Some(seg_base) = repr.children().find(|n| n.tag_name().name() == obfstr!("SegmentBase"))
                    .or_else(|| adaptation_set.children().find(|n| n.tag_name().name() == obfstr!("SegmentBase")))
                {
                    // SegmentBase means single segment (the BaseURL itself)
                    let init = seg_base
                        .children()
                        .find(|n| n.tag_name().name() == obfstr!("Initialization"))
                        .and_then(|init| init.attribute(obfstr!("sourceURL")))
                        .map(|u| resolve_url(&repr_base, u));
                    (init, vec![repr_base.clone()])
                } else {
                    // Fallback: just use the base URL as a single segment
                    (None, vec![repr_base.clone()])
                };

                let track = DashTrack {
                    id: id.clone(),
                    bandwidth,
                    height,
                    width,
                    mime_type: mime.clone(),
                    codecs,
                    init_url,
                    segment_urls,
                    label,
                    is_drm: has_drm,
                };

                let is_video = mime.starts_with(obfstr!("video"))
                    || as_content_type == obfstr!("video")
                    || (height.is_some() && !mime.starts_with(obfstr!("audio")));
                let is_audio = mime.starts_with(obfstr!("audio")) || as_content_type == obfstr!("audio");

                if is_video {
                    video_tracks.push(track);
                } else if is_audio {
                    audio_tracks.push(track);
                }
            }
        }
    }

    // Sort tracks by bandwidth (ascending)
    video_tracks.sort_by_key(|t| t.bandwidth);
    audio_tracks.sort_by_key(|t| t.bandwidth);

    Ok(DashManifest {
        duration,
        video_tracks,
        audio_tracks,
        is_live,
    })
}

fn parse_segment_template(
    seg_tpl: &roxmltree::Node,
    base_url: &str,
    repr_id: &str,
    bandwidth: u64,
    total_duration: f64,
) -> (Option<String>, Vec<String>) {
    let media_template = seg_tpl.attribute(obfstr!("media")).unwrap_or("");
    let init_template = seg_tpl.attribute(obfstr!("initialization"));
    let timescale = seg_tpl
        .attribute(obfstr!("timescale"))
        .and_then(|t| t.parse::<u64>().ok())
        .unwrap_or(1);
    let start_number = seg_tpl
        .attribute(obfstr!("startNumber"))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1);
    let segment_duration = seg_tpl
        .attribute(obfstr!("duration"))
        .and_then(|d| d.parse::<u64>().ok());

    // Init URL
    let init_url = init_template.map(|tpl| {
        let url = substitute_template(tpl, repr_id, bandwidth, 0, 0);
        resolve_url(base_url, &url)
    });

    // Check for SegmentTimeline
    let timeline = seg_tpl
        .children()
        .find(|n| n.tag_name().name() == obfstr!("SegmentTimeline"));

    let mut segment_urls = vec![];

    if let Some(timeline) = timeline {
        // SegmentTimeline with <S> elements
        let mut time: u64 = 0;
        let mut number = start_number;

        for s_elem in timeline.children().filter(|n| n.tag_name().name() == "S") {
            let t = s_elem
                .attribute("t")
                .and_then(|v| v.parse::<u64>().ok());
            let d = s_elem
                .attribute("d")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            let r = s_elem
                .attribute("r")
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0);

            if let Some(t_val) = t {
                time = t_val;
            }

            let repeat_count = if r < 0 {
                // Negative r means repeat until next S or end
                if total_duration > 0.0 && d > 0 {
                    let remaining = (total_duration * timescale as f64) as u64 - time;
                    (remaining / d) as i64
                } else {
                    0
                }
            } else {
                r
            };

            for _ in 0..=repeat_count {
                let url = substitute_template(media_template, repr_id, bandwidth, number, time);
                segment_urls.push(resolve_url(base_url, &url));
                time += d;
                number += 1;
            }
        }
    } else if let Some(seg_dur) = segment_duration {
        // Fixed segment duration
        if total_duration > 0.0 && seg_dur > 0 {
            let total_ts = (total_duration * timescale as f64) as u64;
            let num_segments = (total_ts + seg_dur - 1) / seg_dur;
            let mut time: u64 = 0;

            for i in 0..num_segments {
                let number = start_number + i;
                let url =
                    substitute_template(media_template, repr_id, bandwidth, number, time);
                segment_urls.push(resolve_url(base_url, &url));
                time += seg_dur;
            }
        }
    }

    (init_url, segment_urls)
}

fn parse_segment_list(
    seg_list: &roxmltree::Node,
    base_url: &str,
) -> (Option<String>, Vec<String>) {
    let init_url = seg_list
        .children()
        .find(|n| n.tag_name().name() == obfstr!("Initialization"))
        .and_then(|init| init.attribute(obfstr!("sourceURL")))
        .map(|u| resolve_url(base_url, u));

    let segment_urls: Vec<String> = seg_list
        .children()
        .filter(|n| n.tag_name().name() == obfstr!("SegmentURL"))
        .filter_map(|seg| seg.attribute(obfstr!("media")))
        .map(|u| resolve_url(base_url, u))
        .collect();

    (init_url, segment_urls)
}

/// Substitute DASH template variables.
/// Handles: $RepresentationID$, $Number$, $Number%0Nd$, $Bandwidth$, $Time$
fn substitute_template(
    template: &str,
    repr_id: &str,
    bandwidth: u64,
    number: u64,
    time: u64,
) -> String {
    let mut result = template.to_string();
    result = result.replace(obfstr!("$RepresentationID$"), repr_id);
    result = result.replace(obfstr!("$Bandwidth$"), &bandwidth.to_string());
    result = result.replace(obfstr!("$Time$"), &time.to_string());

    // Handle $Number$ and $Number%0Nd$ (zero-padded)
    if result.contains(obfstr!("$Number")) {
        // Check for format specifier like $Number%05d$
        if let Some(start) = result.find(obfstr!("$Number%")) {
            let end = result[start + 8..].find('$').map(|e| start + 8 + e);
            if let Some(end) = end {
                let format_spec = &result[start + 8..end];
                // Parse format like "05d" -> width=5
                if let Some(width_str) = format_spec.strip_suffix('d') {
                    let width: usize = width_str.trim_start_matches('0').parse().unwrap_or(1);
                    let formatted = format!("{:0>width$}", number, width = width);
                    result = format!("{}{}{}", &result[..start], formatted, &result[end + 1..]);
                }
            }
        }
        result = result.replace(obfstr!("$Number$"), &number.to_string());
    }

    result
}

/// Parse ISO 8601 duration (e.g., "PT1H30M15.5S") to seconds
fn parse_iso_duration(s: &str) -> f64 {
    let s = s.strip_prefix("P").unwrap_or(s);
    let mut total = 0.0;
    let mut current = String::new();
    let mut in_time = false;

    for c in s.chars() {
        match c {
            'T' => {
                in_time = true;
            }
            'H' if in_time => {
                total += current.parse::<f64>().unwrap_or(0.0) * 3600.0;
                current.clear();
            }
            'M' if in_time => {
                total += current.parse::<f64>().unwrap_or(0.0) * 60.0;
                current.clear();
            }
            'S' if in_time => {
                total += current.parse::<f64>().unwrap_or(0.0);
                current.clear();
            }
            'D' => {
                total += current.parse::<f64>().unwrap_or(0.0) * 86400.0;
                current.clear();
            }
            _ => {
                current.push(c);
            }
        }
    }

    total
}

fn find_base_url(node: &roxmltree::Node, parent_base: &str) -> String {
    node.children()
        .find(|n| n.tag_name().name() == obfstr!("BaseURL"))
        .and_then(|n| n.text())
        .map(|text| resolve_url(parent_base, text))
        .unwrap_or_else(|| parent_base.to_string())
}

fn resolve_url(base: &str, relative: &str) -> String {
    if relative.starts_with(obfstr!("http://")) || relative.starts_with(obfstr!("https://")) {
        return relative.to_string();
    }
    Url::parse(base)
        .and_then(|b| b.join(relative))
        .map(|u| u.to_string())
        .unwrap_or_else(|_| format!("{}/{}", base.trim_end_matches('/'), relative))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso_duration() {
        assert!((parse_iso_duration("PT1H30M15.5S") - 5415.5).abs() < 0.001);
        assert!((parse_iso_duration("PT45M") - 2700.0).abs() < 0.001);
        assert!((parse_iso_duration("PT30S") - 30.0).abs() < 0.001);
        assert!((parse_iso_duration("P1DT2H") - 93600.0).abs() < 0.001);
    }

    #[test]
    fn test_substitute_template() {
        let result = substitute_template(
            "segment_$RepresentationID$_$Number%05d$.m4s",
            "1", 500000, 42, 0,
        );
        assert_eq!(result, "segment_1_00042.m4s");
    }
}
