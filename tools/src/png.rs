//! Indexed 8-bit PNG, the only kind the Vita accepts for LiveArea files:
//! a median-cut palette of at most 256 colors and Floyd-Steinberg dithering
//! with the carried error capped. Without dithering, 256 colors showed the
//! backgrounds' gradients and glows as bands and blotches; uncapped, the
//! error built up in the dark lower half of the splash into reddish dots.

use std::collections::HashMap;

/// The most rounding error a pixel carries from its neighbors, per channel.
const ERROR_CAP: f32 = 12.0;

fn channel(color: u32, index: usize) -> u32 {
    (color >> (16 - index * 8)) & 255
}

struct Box {
    colors: Vec<u32>,
    min: [u32; 3],
    max: [u32; 3],
}

fn box_of(colors: Vec<u32>) -> Box {
    let (mut min, mut max) = ([255; 3], [0; 3]);
    for &color in &colors {
        for c in 0..3 {
            min[c] = min[c].min(channel(color, c));
            max[c] = max[c].max(channel(color, c));
        }
    }
    Box { colors, min, max }
}

/// Median cut: split the box with the widest channel at its median pixel
/// until there are `limit` boxes, then average each one, each color weighed
/// by how many pixels use it. `order` is the colors in the order first seen.
fn palette(order: &[u32], counts: &HashMap<u32, u32>, limit: usize) -> Vec<u32> {
    let mut boxes = vec![box_of(order.to_vec())];
    while boxes.len() < limit {
        let (mut widest, mut widest_range, mut widest_channel) = (None, 0, 0);
        for (index, b) in boxes.iter().enumerate() {
            if b.colors.len() < 2 {
                continue;
            }
            for c in 0..3 {
                let range = b.max[c] - b.min[c];
                if range > widest_range {
                    (widest, widest_range, widest_channel) = (Some(index), range, c);
                }
            }
        }
        let Some(widest) = widest else { break };
        let mut colors = boxes.remove(widest).colors;
        colors.sort_by_key(|&color| channel(color, widest_channel));
        let total: u64 = colors.iter().map(|c| counts[c] as u64).sum();
        let (mut seen, mut cut) = (0u64, 1);
        for (index, color) in colors.iter().enumerate().take(colors.len() - 1) {
            seen += counts[color] as u64;
            cut = index + 1;
            if seen * 2 >= total {
                break;
            }
        }
        let rest = colors.split_off(cut);
        boxes.insert(widest, box_of(rest));
        boxes.insert(widest, box_of(colors));
    }
    boxes
        .iter()
        .map(|b| {
            let mut weight = 0u64;
            let mut sum = [0u64; 3];
            for color in &b.colors {
                let count = counts[color] as u64;
                weight += count;
                for (c, total) in sum.iter_mut().enumerate() {
                    *total += channel(*color, c) as u64 * count;
                }
            }
            let average = |total: u64| (total as f64 / weight.max(1) as f64).round() as u32;
            (average(sum[0]) << 16) | (average(sum[1]) << 8) | average(sum[2])
        })
        .collect()
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { 0xedb8_8320 ^ (crc >> 1) } else { crc >> 1 };
        }
    }
    crc ^ 0xffff_ffff
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// Opaque RGBA pixels as an indexed, non-interlaced 8-bit PNG.
pub fn indexed(rgba: &[u8], width: usize, height: usize) -> Vec<u8> {
    let pixels: Vec<u32> = rgba.chunks_exact(4).map(|p| ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | p[2] as u32).collect();
    let mut counts: HashMap<u32, u32> = HashMap::new();
    let mut order = Vec::new();
    for &color in &pixels {
        let count = counts.entry(color).or_insert(0);
        if *count == 0 {
            order.push(color);
        }
        *count += 1;
    }
    let colors = palette(&order, &counts, 256);
    let mut nearest: HashMap<u32, u8> = HashMap::new();
    let mut lookup = |color: u32| -> u8 {
        *nearest.entry(color).or_insert_with(|| {
            let mut best = (u32::MAX, 0);
            for (index, &entry) in colors.iter().enumerate() {
                let d = |c| (channel(color, c) as i32 - channel(entry, c) as i32).pow(2) as u32;
                let distance = d(0) + d(1) + d(2);
                if distance < best.0 {
                    best = (distance, index);
                }
            }
            best.1 as u8
        })
    };
    let mut scanlines = vec![0u8; (width + 1) * height];
    let mut error = vec![0f32; width * height * 3];
    for y in 0..height {
        for x in 0..width {
            let at = y * width + x;
            let mut wanted = [0u32; 3];
            for (c, value) in wanted.iter_mut().enumerate() {
                let carried = error[at * 3 + c].clamp(-ERROR_CAP, ERROR_CAP);
                *value = (channel(pixels[at], c) as f32 + carried).round().clamp(0.0, 255.0) as u32;
            }
            let index = lookup((wanted[0] << 16) | (wanted[1] << 8) | wanted[2]);
            scanlines[y * (width + 1) + 1 + x] = index;
            let chosen = colors[index as usize];
            for c in 0..3 {
                let miss = wanted[c] as f32 - channel(chosen, c) as f32;
                let mut spread = |dx: isize, dy: usize, share: f32| {
                    let nx = x as isize + dx;
                    let ny = y + dy;
                    if nx < 0 || nx >= width as isize || ny >= height {
                        return;
                    }
                    error[(ny * width + nx as usize) * 3 + c] += miss * share;
                };
                spread(1, 0, 7.0 / 16.0);
                spread(-1, 1, 3.0 / 16.0);
                spread(0, 1, 5.0 / 16.0);
                spread(1, 1, 1.0 / 16.0);
            }
        }
    }
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&(width as u32).to_be_bytes());
    header.extend_from_slice(&(height as u32).to_be_bytes());
    header.extend_from_slice(&[8, 3, 0, 0, 0]); // 8-bit, indexed
    let table: Vec<u8> = colors.iter().flat_map(|&c| [channel(c, 0) as u8, channel(c, 1) as u8, channel(c, 2) as u8]).collect();
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"PLTE", &table);
    chunk(&mut out, b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&scanlines, 9));
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn crc_matches_the_png_specification() {
        // The CRC of an empty IEND chunk, as every PNG ends.
        assert_eq!(super::crc32(b"IEND"), 0xae42_6082);
    }

    #[test]
    fn a_two_color_picture_keeps_both_colors() {
        let rgba = [255, 0, 0, 255, 0, 0, 255, 255].repeat(8);
        let png = super::indexed(&rgba, 4, 4);
        let plte = png.windows(4).position(|w| w == b"PLTE").unwrap();
        let length = u32::from_be_bytes(png[plte - 4..plte].try_into().unwrap());
        assert_eq!(length, 6);
    }
}
