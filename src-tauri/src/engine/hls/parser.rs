use url::Url;

#[derive(Debug, Clone)]
pub struct HlsMasterPlaylist {
    pub variants: Vec<HlsVariant>,
}

#[derive(Debug, Clone)]
pub struct HlsVariant {
    pub url: String,
    pub bandwidth: u64,
    pub height: Option<u32>,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct HlsMediaPlaylist {
    pub segments: Vec<HlsSegment>,
    pub encryption: Option<HlsEncryption>,
    pub init_map_url: Option<String>,
    pub total_duration: f64,
    pub is_live: bool,
    pub media_sequence: u64,
}

#[derive(Debug, Clone)]
pub struct HlsSegment {
    pub url: String,
    pub duration: f64,
    pub sequence: u64,
    pub iv: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct HlsEncryption {
    pub method: String,
    pub key_uri: String,
    pub iv: Option<Vec<u8>>,
}

pub enum M3u8Result {
    Master(HlsMasterPlaylist),
    Media(HlsMediaPlaylist),
}

pub fn parse_m3u8(content: &str, base_url: &str) -> Result<M3u8Result, String> {
    let lines: Vec<&str> = content.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    if !lines.first().map(|l| l.contains("#EXTM3U")).unwrap_or(false) {
        return Err("Not a valid M3U8 playlist".into());
    }

    if lines.iter().any(|l| l.contains("#EXT-X-STREAM-INF")) {
        Ok(M3u8Result::Master(parse_master(&lines, base_url)?))
    } else {
        Ok(M3u8Result::Media(parse_media(&lines, base_url)?))
    }
}

fn parse_master(lines: &[&str], base_url: &str) -> Result<HlsMasterPlaylist, String> {
    let mut variants = vec![];
    let mut i = 0;
    while i < lines.len() {
        if lines[i].contains("#EXT-X-STREAM-INF") {
            let tag = lines[i];
            let bandwidth = parse_attr(tag, "BANDWIDTH").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            let resolution = parse_attr(tag, "RESOLUTION");
            i += 1;
            while i < lines.len() && lines[i].starts_with('#') { i += 1; }
            if i < lines.len() {
                let variant_url = resolve_url(base_url, lines[i]);
                let height = resolution.as_ref().and_then(|r| r.split('x').last()?.parse::<u32>().ok());
                let label = height.map(|h| format!("{}p", h)).unwrap_or_else(|| format!("{}k", bandwidth / 1000));
                variants.push(HlsVariant { url: variant_url, bandwidth, height, label });
            }
        }
        i += 1;
    }
    variants.sort_by_key(|v| v.bandwidth);
    Ok(HlsMasterPlaylist { variants })
}

fn parse_media(lines: &[&str], base_url: &str) -> Result<HlsMediaPlaylist, String> {
    let mut segments = vec![];
    let mut encryption: Option<HlsEncryption> = None;
    let mut current_key: Option<HlsEncryption> = None;
    let mut init_map_url: Option<String> = None;
    let mut media_sequence: u64 = 0;
    let mut seg_idx: u64 = 0;
    let mut total_duration: f64 = 0.0;
    let mut current_duration: f64 = 0.0;

    for line in lines {
        if line.contains("#EXT-X-MEDIA-SEQUENCE") {
            if let Some(val) = line.split(':').last() {
                media_sequence = val.trim().parse().unwrap_or(0);
            }
        }
    }

    for line in lines {
        if line.contains("#EXT-X-MAP") {
            if let Some(uri) = parse_attr(line, "URI") {
                init_map_url = Some(resolve_url(base_url, &uri));
            }
            continue;
        }

        if line.contains("#EXT-X-KEY") {
            let method = parse_attr(line, "METHOD").unwrap_or_default();
            if method == "NONE" {
                current_key = None;
            } else if method == "AES-128" {
                let uri = parse_attr(line, "URI").unwrap_or_default();
                let iv = parse_attr(line, "IV").and_then(|h| parse_hex_iv(&h));
                let key = HlsEncryption { method, key_uri: resolve_url(base_url, &uri), iv: iv.clone() };
                if encryption.is_none() { encryption = Some(key.clone()); }
                current_key = Some(key);
            }
            continue;
        }

        if line.starts_with("#EXTINF") {
            if let Some(dur_str) = line.split(':').last() {
                current_duration = dur_str.trim_end_matches(',').parse::<f64>().unwrap_or(0.0);
                total_duration += current_duration;
            }
            continue;
        }

        if line.starts_with('#') { continue; }

        let seg_url = resolve_url(base_url, line);
        segments.push(HlsSegment {
            url: seg_url,
            duration: current_duration,
            sequence: media_sequence + seg_idx,
            iv: current_key.as_ref().and_then(|k| k.iv.clone()),
        });
        seg_idx += 1;
        current_duration = 0.0;
    }

    let full_text = lines.join("\n");
    let is_live = !full_text.contains("#EXT-X-ENDLIST")
        && !full_text.contains("#EXT-X-PLAYLIST-TYPE:VOD")
        && total_duration < 120.0
        && segments.len() < 30;

    Ok(HlsMediaPlaylist { segments, encryption, init_map_url, total_duration, is_live, media_sequence })
}

fn parse_attr(tag: &str, name: &str) -> Option<String> {
    let search = format!("{}=", name);
    let start = tag.find(&search)? + search.len();
    let rest = &tag[start..];
    if rest.starts_with('"') {
        let end = rest[1..].find('"')? + 1;
        Some(rest[1..end].to_string())
    } else {
        let end = rest.find(',').unwrap_or(rest.len());
        Some(rest[..end].to_string())
    }
}

fn resolve_url(base: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }
    Url::parse(base)
        .and_then(|b| b.join(relative))
        .map(|u| u.to_string())
        .unwrap_or_else(|_| format!("{}/{}", base.trim_end_matches('/'), relative))
}

fn parse_hex_iv(hex: &str) -> Option<Vec<u8>> {
    let hex = hex.strip_prefix("0x").or(hex.strip_prefix("0X")).unwrap_or(hex);
    let padded = format!("{:0>32}", hex);
    (0..16).map(|i| u8::from_str_radix(&padded[i*2..i*2+2], 16).ok()).collect()
}
