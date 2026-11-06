//! A decoded picture cropped to the screen's shape and averaged down to a
//! backdrop texture (`backdrop.rs`). No system calls, so its tests run on the
//! desktop (`platform-tests`).

/// Width, height and RGBA pixels.
pub type Pixels = (u32, u32, Vec<u8>);

/// The largest texture: 512x256 texels. A picture about twice the screen's
/// size (960x544) gives 480x272, the same number of texels.
pub const OUT_W: u32 = 512;
pub const OUT_H: u32 = 256;
/// Smallest texture produced for a small source image.
pub const OUT_W_MIN: u32 = 64;
/// Screen shape the picture is cropped to (480:272).
const ASPECT_W: u64 = 30;
const ASPECT_H: u64 = 17;

/// Crop `buffer` (rows `line_size` bytes apart, `channels` bytes a pixel:
/// gray, gray and alpha, RGB or RGBA) to the largest centered 30:17
/// rectangle and average it into an opaque RGBA texture. None for a picture
/// too small or a buffer too short.
pub fn shrink(buffer: &[u8], line_size: usize, channels: usize, width: u32, height: u32) -> Option<Pixels> {
    let (crop_w, crop_h) = if width as u64 * ASPECT_H > height as u64 * ASPECT_W {
        ((height as u64 * ASPECT_W / ASPECT_H) as u32, height)
    } else {
        (width, (width as u64 * ASPECT_H / ASPECT_W) as u32)
    };
    let origin_x = (width - crop_w) / 2;
    let origin_y = (height - crop_h) / 2;
    let crop = Crop { buffer, line_size, channels, x: origin_x as usize, y: origin_y as usize, w: crop_w, h: crop_h };
    // A picture between 1.5 and 2 times the largest texture's width, at no
    // more texels than it: each texel is the average of a 2x2 block. The
    // general path below took 200-250 ms for a 960x544 picture on hardware.
    let (half_w, half_h) = (crop_w / 2, crop_h / 2);
    if half_w * 4 >= OUT_W * 3 && half_w * half_h <= OUT_W * OUT_H {
        return crop.halve(half_w, half_h);
    }
    // The filter averages source pixels into each texel, so the texture is
    // never larger than the cropped picture.
    let (mut out_w, mut out_h) = (OUT_W, OUT_H);
    while out_w > crop_w || out_h > crop_h {
        out_w /= 2;
        out_h /= 2;
    }
    if out_w < OUT_W_MIN {
        return None;
    }
    crop.average(out_w, out_h)
}

struct Crop<'a> {
    buffer: &'a [u8],
    line_size: usize,
    channels: usize,
    x: usize,
    y: usize,
    w: u32,
    h: u32,
}

impl Crop<'_> {
    /// Row `y` of the crop, `w` pixels.
    fn row(&self, y: usize) -> Option<&[u8]> {
        let start = (self.y + y) * self.line_size + self.x * self.channels;
        self.buffer.get(start..start + self.w as usize * self.channels)
    }

    /// A pixel's red, green and blue (gray repeated).
    fn rgb(&self, pixel: &[u8]) -> (u32, u32, u32) {
        if self.channels >= 3 {
            (pixel[0] as u32, pixel[1] as u32, pixel[2] as u32)
        } else {
            (pixel[0] as u32, pixel[0] as u32, pixel[0] as u32)
        }
    }

    /// Each texel the average of the 2x2 block under it.
    fn halve(&self, out_w: u32, out_h: u32) -> Option<Pixels> {
        let pair = self.channels * 2;
        let mut rgba = Vec::with_capacity((out_w * out_h * 4) as usize);
        for y in 0..out_h as usize {
            let (top, bottom) = (self.row(y * 2)?, self.row(y * 2 + 1)?);
            for (a, b) in top.chunks_exact(pair).zip(bottom.chunks_exact(pair)).take(out_w as usize) {
                let (c, d) = (&a[self.channels..], &b[self.channels..]);
                let (r0, g0, b0) = self.rgb(a);
                let (r1, g1, b1) = self.rgb(c);
                let (r2, g2, b2) = self.rgb(b);
                let (r3, g3, b3) = self.rgb(d);
                rgba.extend_from_slice(&[((r0 + r1 + r2 + r3 + 2) / 4) as u8, ((g0 + g1 + g2 + g3 + 2) / 4) as u8, ((b0 + b1 + b2 + b3 + 2) / 4) as u8, 255]);
            }
        }
        Some((out_w, out_h, rgba))
    }

    /// Each texel the average of the source pixels that fall in it.
    fn average(&self, out_w: u32, out_h: u32) -> Option<Pixels> {
        let cells = (out_w * out_h) as usize;
        let mut sums = vec![0u32; cells * 4];
        let mut counts = vec![0u32; cells];
        // Each source column's texel column, worked out once: a 64-bit
        // division per pixel is a library call on the Vita's 32-bit CPU, and
        // it was most of a backdrop's 0.5–1.4 s on hardware.
        let columns: Vec<u32> = (0..self.w).map(|x| (x as u64 * out_w as u64 / self.w as u64) as u32).collect();
        for y in 0..self.h as usize {
            let row = self.row(y)?;
            let base = (y as u64 * out_h as u64 / self.h as u64) as usize * out_w as usize;
            for (pixel, &column) in row.chunks_exact(self.channels).zip(&columns) {
                let (r, g, b) = self.rgb(pixel);
                let cell = base + column as usize;
                let sum = &mut sums[cell * 4..cell * 4 + 3];
                sum[0] += r;
                sum[1] += g;
                sum[2] += b;
                counts[cell] += 1;
            }
        }
        // A background is drawn opaque, so the picture's own alpha is dropped.
        let mut rgba = Vec::with_capacity(cells * 4);
        for cell in 0..cells {
            let count = counts[cell].max(1);
            for channel in 0..3 {
                rgba.push((sums[cell * 4 + channel] / count) as u8);
            }
            rgba.push(255);
        }
        Some((out_w, out_h, rgba))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `w`x`h` RGB picture whose red is its column and green its row.
    fn ramp(w: u32, h: u32) -> Vec<u8> {
        (0..h).flat_map(|y| (0..w).flat_map(move |x| [(x % 256) as u8, (y % 256) as u8, 7])).collect()
    }

    #[test]
    fn a_screen_sized_picture_halves_into_2x2_averages() {
        let (w, h, rgba) = shrink(&ramp(960, 544), 960 * 3, 3, 960, 544).unwrap();
        assert_eq!((w, h, rgba.len()), (480, 272, 480 * 272 * 4));
        // Texel (1, 2): columns 2-3 and rows 4-5, rounded.
        let texel = &rgba[(2 * 480 + 1) * 4..(2 * 480 + 2) * 4];
        assert_eq!(texel, [3, 5, 7, 255]);
    }

    #[test]
    fn a_livearea_picture_is_cropped_then_halved() {
        // 840x500 crops to 840x476 (30:17), from row 12.
        let (w, h, rgba) = shrink(&ramp(840, 500), 840 * 3, 3, 840, 500).unwrap();
        assert_eq!((w, h), (420, 238));
        assert_eq!(&rgba[..4], [1, 13, 7, 255]);
    }

    #[test]
    fn a_large_picture_averages_into_the_largest_texture() {
        let (w, h, _) = shrink(&ramp(1920, 1088), 1920 * 3, 3, 1920, 1088).unwrap();
        assert_eq!((w, h), (OUT_W, OUT_H));
    }

    #[test]
    fn a_small_picture_gives_a_smaller_texture_or_none() {
        assert_eq!(shrink(&ramp(480, 272), 480 * 3, 3, 480, 272).map(|p| (p.0, p.1)), Some((256, 128)));
        assert!(shrink(&ramp(60, 34), 60 * 3, 3, 60, 34).is_none());
    }

    #[test]
    fn gray_and_alpha_pictures_read_their_channels() {
        let gray: Vec<u8> = (0..960 * 544).flat_map(|_| [200u8, 9]).collect();
        let (_, _, rgba) = shrink(&gray, 960 * 2, 2, 960, 544).unwrap();
        assert_eq!(&rgba[..4], [200, 200, 200, 255]);
    }

    #[test]
    fn a_short_buffer_gives_none() {
        assert!(shrink(&ramp(960, 100), 960 * 3, 3, 960, 544).is_none());
    }
}
