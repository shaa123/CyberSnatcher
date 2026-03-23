use obfstr::obfstr;
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
    pub init_map_byterange: Option<ByteRange>,
    pub total_duration: f64,
    pub is_live: bool,
    pub media_sequence: u64,
    pub has_discontinuity: bool,
}

#[derive(Debug, Clone)]
pub struct HlsSegment {
    pub url: String,
    pub duration: f64,
    pub sequence: u64,
    pub iv: Option<Vec<u8>>,
    pub byterange: Option<ByteRange>,
    pub is_discontinuity: bool,
}

#[derive(Debug, Clone)]
pub struct ByteRange {
    pub length: u64,
    pub offset: Option<u64>,
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

    if !lines.first().map(|l| l.contains(obfstr!("#EXTM3U"))).unwrap_or(false) {
        return Err(obfstr!("Not a valid M3U8 playlist").into());
    }

    if lines.iter().any(|l| l.contains(obfstr!("#EXT-X-STREAM-INF"))) {
        Ok(M3u8Result::Master(parse_master(&lines, base_url)?))
    } else {
        Ok(M3u8Result::Media(parse_media(&lines, base_url)?))
    }
}

fn parse_master(lines: &[&str], base_url: &str) -> Result<HlsMasterPlaylist, String> {
    let mut variants = vec![];
    let mut i = 0;
    while i < lines.len() {
        if lines[i].contains(obfstr!("#EXT-X-STREAM-INF")) {
            let tag = lines[i];
            let bandwidth = parse_attr(tag, obfstr!("BANDWIDTH")).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            let resolution = parse_attr(tag, obfstr!("RESOLUTION"));
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
    let mut init_map_byterange: Option<ByteRange> = None;
    let mut media_sequence: u64 = 0;
    let mut seg_idx: u64 = 0;
    let mut total_duration: f64 = 0.0;
    let mut current_duration: f64 = 0.0;
    let mut current_byterange: Option<ByteRange> = None;
    let mut has_discontinuity = false;
    let mut next_is_discontinuity = false;
    let mut last_byterange_offset: u64 = 0;

    for line in lines {
        if line.contains(obfstr!("#EXT-X-MEDIA-SEQUENCE")) {
            if let Some(val) = line.split(':').last() {
                media_sequence = val.trim().parse().unwrap_or(0);
            }
        }
    }

    for line in lines {
        // #EXT-X-MAP (initialization segment for fMP4)
        if line.contains(obfstr!("#EXT-X-MAP")) {
            if let Some(uri) = parse_attr(line, obfstr!("URI")) {
                init_map_url = Some(resolve_url(base_url, &uri));
            }
            if let Some(br) = parse_attr(line, obfstr!("BYTERANGE")) {
                init_map_byterange = parse_byterange_value(&br, 0);
            }
            continue;
        }

        // #EXT-X-KEY
        if line.contains(obfstr!("#EXT-X-KEY")) {
            let method = parse_attr(line, obfstr!("METHOD")).unwrap_or_default();
            if method == obfstr!("NONE") {
                current_key = None;
            } else if method == obfstr!("AES-128") {
                let uri = parse_attr(line, obfstr!("URI")).unwrap_or_default();
                let iv = parse_attr(line, obfstr!("IV")).and_then(|h| parse_hex_iv(&h));
                let key = HlsEncryption { method, key_uri: resolve_url(base_url, &uri), iv: iv.clone() };
                if encryption.is_none() { encryption = Some(key.clone()); }
                current_key = Some(key);
            } else if method == obfstr!("SAMPLE-AES") || method == obfstr!("SAMPLE-AES-CTR") {
                // Warn: unsupported encryption, but continue parsing
                log::warn!("{}{}{}", obfstr!("HLS: Unsupported encryption method: "), method, obfstr!(". Segments may fail to decrypt."));
                let uri = parse_attr(line, obfstr!("URI")).unwrap_or_default();
                let iv = parse_attr(line, obfstr!("IV")).and_then(|h| parse_hex_iv(&h));
                let key = HlsEncryption { method, key_uri: resolve_url(base_url, &uri), iv };
                if encryption.is_none() { encryption = Some(key.clone()); }
                current_key = Some(key);
            }
            continue;
        }

        // #EXT-X-DISCONTINUITY
        if line.contains(obfstr!("#EXT-X-DISCONTINUITY")) {
            has_discontinuity = true;
            next_is_discontinuity = true;
            continue;
        }

        // #EXT-X-BYTERANGE
        if line.starts_with(obfstr!("#EXT-X-BYTERANGE")) {
            if let Some(val) = line.split(':').last() {
                current_byterange = parse_byterange_value(val.trim(), last_byterange_offset);
            }
            continue;
        }

        // #EXTINF
        if line.starts_with(obfstr!("#EXTINF")) {
            if let Some(dur_str) = line.split(':').last() {
                current_duration = dur_str.trim_end_matches(',').parse::<f64>().unwrap_or(0.0);
                total_duration += current_duration;
            }
            continue;
        }

        if line.starts_with('#') { continue; }

        // Segment URL
        let seg_url = resolve_url(base_url, line);

        // Track byterange offsets for consecutive ranges on the same URL
        if let Some(ref br) = current_byterange {
            let offset = br.offset.unwrap_or(0);
            last_byterange_offset = offset + br.length;
        }

        segments.push(HlsSegment {
            url: seg_url,
            duration: current_duration,
            sequence: media_sequence + seg_idx,
            iv: current_key.as_ref().and_then(|k| k.iv.clone()),
            byterange: current_byterange.take(),
            is_discontinuity: next_is_discontinuity,
        });
        seg_idx += 1;
        current_duration = 0.0;
        next_is_discontinuity = false;
    }

    let full_text = lines.join("\n");
    // Per HLS spec (RFC 8216 §4.3.3.4): absence of #EXT-X-ENDLIST is the
    // authoritative signal that a playlist is live.
    let is_live = !full_text.contains(obfstr!("#EXT-X-ENDLIST"))
        && !full_text.contains(obfstr!("#EXT-X-PLAYLIST-TYPE:VOD"));

    Ok(HlsMediaPlaylist {
        segments,
        encryption,
        init_map_url,
        init_map_byterange,
        total_duration,
        is_live,
        media_sequence,
        has_discontinuity,
    })
}

/// Parse a byterange value like "1024@0" or "1024" into ByteRange
fn parse_byterange_value(val: &str, default_offset: u64) -> Option<ByteRange> {
    let parts: Vec<&str> = val.split('@').collect();
    let length = parts[0].parse::<u64>().ok()?;
    let offset = if parts.len() > 1 {
        parts[1].parse::<u64>().ok()
    } else {
        Some(default_offset)
    };
    Some(ByteRange { length, offset })
}

fn parse_attr(tag: &str, name: &str) -> Option<String> {
    let search = format!("{}=", name);
    // Require the match to be at the start of the attribute list or immediately
    // after a comma, so that "IV=" does not substring-match inside "KEYFORMATVERSIONS=".
    let pos = tag.find(&search).filter(|&p| p == 0 || tag.as_bytes()[p - 1] == b',')?;
    let start = pos + search.len();
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
