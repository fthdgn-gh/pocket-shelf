//! Accent color of a decoded icon (cargo feature `installed-apps`).
//!
//! The launcher can draw a box behind an icon that has transparent parts, in
//! a color taken from the icon. This module finds that color when the host
//! decodes the icon and keeps it by texture handle for the guest to ask for.
//!
//! The color is the icon's strongest hue: pixels vote for one of 24 hue bins
//! with their colorfulness as the weight, and the winning bin and its two
//! neighbors are averaged. Gray, black and white pixels do not vote; an icon
//! with almost no color gets its average gray.

use std::vec::Vec;

/// Bit set in the packed result when the icon has transparent parts.
pub const HAS_ALPHA: i32 = 1 << 24;

const BINS: usize = 24;
/// Pixels looked at, at most. Larger icons are sampled on an even stride.
const SAMPLES_MAX: usize = 16 * 1024;

/// Packed accent per texture handle: `HAS_ALPHA | 0xRRGGBB`.
static mut ACCENTS: Vec<(i32, i32)> = Vec::new();

/// The accent of tightly packed RGBA pixels, packed as `HAS_ALPHA | 0xRRGGBB`.
pub fn of(rgba: &[u8]) -> i32 {
    let pixels = rgba.len() / 4;
    if pixels == 0 {
        return 0;
    }
    let stride = (pixels / SAMPLES_MAX).max(1);
    // Per bin: weight, then red, green and blue each multiplied by the weight.
    let mut bins = [[0u64; 4]; BINS];
    let mut gray = [0u64; 4];
    let (mut sampled, mut see_through, mut colored) = (0u32, 0u32, 0u32);
    for index in (0..pixels).step_by(stride) {
        let pixel = &rgba[index * 4..index * 4 + 4];
        sampled += 1;
        if pixel[3] < 250 {
            see_through += 1;
        }
        if pixel[3] < 128 {
            continue;
        }
        let (r, g, b) = (pixel[0] as i32, pixel[1] as i32, pixel[2] as i32);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let spread = max - min;
        // Too dark or too close to gray to have a hue worth counting.
        if max < 51 || spread * 5 < max {
            gray[0] += 1;
            gray[1] += r as u64;
            gray[2] += g as u64;
            gray[3] += b as u64;
            continue;
        }
        let sixth = if max == r {
            ((g - b) as f32 / spread as f32).rem_euclid(6.0)
        } else if max == g {
            (b - r) as f32 / spread as f32 + 2.0
        } else {
            (r - g) as f32 / spread as f32 + 4.0
        };
        let bin = ((sixth / 6.0 * BINS as f32) as usize).min(BINS - 1);
        let weight = spread as u64;
        bins[bin][0] += weight;
        bins[bin][1] += r as u64 * weight;
        bins[bin][2] += g as u64 * weight;
        bins[bin][3] += b as u64 * weight;
        colored += 1;
    }
    let opaque = colored + gray[0] as u32;
    let sums = if opaque > 0 && colored * 33 >= opaque {
        // The strongest bin with its neighbors, so a hue that straddles two
        // bins is not split.
        let around = |bin: usize| [(bin + BINS - 1) % BINS, bin, (bin + 1) % BINS];
        let weight_around = |bin: usize| around(bin).iter().map(|&at| bins[at][0]).sum::<u64>();
        let best = (0..BINS).max_by_key(|&bin| weight_around(bin)).unwrap_or(0);
        let mut sums = [0u64; 4];
        for at in around(best) {
            for part in 0..4 {
                sums[part] += bins[at][part];
            }
        }
        sums
    } else {
        gray
    };
    let total = sums[0].max(1);
    let channel = |sum: u64| ((sum / total) as i32).min(255);
    let color = (channel(sums[1]) << 16) | (channel(sums[2]) << 8) | channel(sums[3]);
    // A few soft edge pixels do not make an icon see-through.
    if see_through * 50 > sampled {
        color | HAS_ALPHA
    } else {
        color
    }
}

/// Remember the accent of the pixels uploaded as `handle`.
pub unsafe fn record(handle: i32, rgba: &[u8]) {
    if handle >= 0 {
        ACCENTS.push((handle, of(rgba)));
    }
}

/// The accent recorded for a texture handle, or -1 when there is none.
pub unsafe fn lookup(handle: i32) -> i32 {
    ACCENTS
        .iter()
        .find(|(held, _)| *held == handle)
        .map_or(-1, |(_, accent)| *accent)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(pixels: &[([u8; 4], usize)]) -> Vec<u8> {
        let mut rgba = Vec::new();
        for (pixel, count) in pixels {
            for _ in 0..*count {
                rgba.extend_from_slice(pixel);
            }
        }
        rgba
    }

    #[test]
    fn opaque_single_color() {
        assert_eq!(of(&image(&[([200, 30, 30, 255], 100)])), 0xc81e1e);
    }

    #[test]
    fn transparent_background_sets_the_flag() {
        let accent = of(&image(&[([0, 0, 0, 0], 60), ([20, 60, 220, 255], 40)]));
        assert_eq!(accent, HAS_ALPHA | 0x143cdc);
    }

    #[test]
    fn a_few_soft_pixels_are_not_transparency() {
        let accent = of(&image(&[([20, 60, 220, 255], 99), ([20, 60, 220, 120], 1)]));
        assert_eq!(accent & HAS_ALPHA, 0);
    }

    #[test]
    fn the_strongest_hue_wins_over_gray_and_weaker_hues() {
        let accent = of(&image(&[
            ([255, 255, 255, 255], 50),
            ([10, 10, 10, 255], 20),
            ([230, 40, 40, 255], 20),
            ([40, 200, 60, 255], 10),
        ]));
        assert_eq!(accent, 0xe62828);
    }

    #[test]
    fn an_icon_without_color_gets_its_gray() {
        assert_eq!(of(&image(&[([200, 200, 200, 255], 50), ([100, 100, 100, 255], 50)])), 0x969696);
    }

    #[test]
    fn empty_input_is_black() {
        assert_eq!(of(&[]), 0);
    }
}
