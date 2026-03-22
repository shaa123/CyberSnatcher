// mp4patch.rs — MP4 Duration Patching
// Finds mvhd/mdhd/tkhd boxes in an MP4 file and writes correct duration.
// Ported from downloader.js patchMp4Duration function.

use std::path::Path;

/// Patch the duration in an MP4 file's mvhd, mdhd, and tkhd boxes.
/// This fixes videos where the duration is 0 or wrong after concatenation.
pub fn patch_mp4_duration(path: &Path, duration_seconds: f64) {
    if duration_seconds <= 0.0 { return; }

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return,
    };

    let mut mp4 = data;
    let mut patched = 0u32;

    // Patch mvhd (Movie Header Box) — contains global duration
    if let Some(pos) = find_box(&mp4, b"mvhd") {
        let base = pos + 4; // skip box type, version byte is here
        if base < mp4.len() && mp4[base] == 0 {
            // Version 0: timescale at offset 12, duration at offset 16
            if base + 20 <= mp4.len() {
                let ts = read_u32(&mp4, base + 12);
                if ts > 0 {
                    let dur = (duration_seconds * ts as f64).round() as u32;
                    write_u32(&mut mp4, base + 16, dur);
                    patched += 1;
                }
            }
        }
    }

    // Patch all mdhd boxes (Media Header Box) — per-track duration
    let mut search_from = 0;
    loop {
        match find_box_from(&mp4, b"mdhd", search_from) {
            Some(pos) => {
                let base = pos + 4;
                if base < mp4.len() && mp4[base] == 0 {
                    if base + 20 <= mp4.len() {
                        let ts = read_u32(&mp4, base + 12);
                        if ts > 0 {
                            let dur = (duration_seconds * ts as f64).round() as u32;
                            write_u32(&mut mp4, base + 16, dur);
                            patched += 1;
                        }
                    }
                }
                search_from = pos + 20;
            }
            None => break,
        }
    }

    // Patch all tkhd boxes (Track Header Box) — per-track duration
    search_from = 0;
    loop {
        match find_box_from(&mp4, b"tkhd", search_from) {
            Some(pos) => {
                let base = pos + 4;
                if base < mp4.len() && mp4[base] == 0 {
                    // Duration is at offset 20 in version 0 tkhd
                    // Uses the movie timescale (from mvhd)
                    if base + 24 <= mp4.len() {
                        // Get movie timescale from mvhd
                        let movie_ts = find_box(&mp4, b"mvhd")
                            .map(|p| read_u32(&mp4, p + 4 + 12))
                            .unwrap_or(90000);
                        if movie_ts > 0 {
                            let dur = (duration_seconds * movie_ts as f64).round() as u32;
                            write_u32(&mut mp4, base + 20, dur);
                            patched += 1;
                        }
                    }
                }
                search_from = pos + 24;
            }
            None => break,
        }
    }

    if patched > 0 {
        let _ = std::fs::write(path, &mp4);
    }
}

/// Find the position of a box type (4-byte name) in MP4 data.
/// Returns the position of the box type bytes (not the size prefix).
fn find_box(data: &[u8], name: &[u8; 4]) -> Option<usize> {
    find_box_from(data, name, 0)
}

fn find_box_from(data: &[u8], name: &[u8; 4], start: usize) -> Option<usize> {
    if data.len() < 8 { return None; }
    for i in start..data.len().saturating_sub(3) {
        if data[i] == name[0] && data[i + 1] == name[1]
            && data[i + 2] == name[2] && data[i + 3] == name[3] {
            return Some(i);
        }
    }
    None
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    if offset + 4 > data.len() { return 0; }
    ((data[offset] as u32) << 24)
        | ((data[offset + 1] as u32) << 16)
        | ((data[offset + 2] as u32) << 8)
        | (data[offset + 3] as u32)
}

fn write_u32(data: &mut [u8], offset: usize, value: u32) {
    if offset + 4 > data.len() { return; }
    data[offset] = ((value >> 24) & 0xFF) as u8;
    data[offset + 1] = ((value >> 16) & 0xFF) as u8;
    data[offset + 2] = ((value >> 8) & 0xFF) as u8;
    data[offset + 3] = (value & 0xFF) as u8;
}
