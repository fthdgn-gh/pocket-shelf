//! DDS textures, as the home screen keeps PlayStation Mobile icons
//! (`ur0:appmeta/<id>/icon0.dds`: a 128-byte header and a 128x128 DXT1 image,
//! 8320 bytes). Decodes DXT1, DXT3, DXT5 and uncompressed 32-bit images, the
//! first level only, to tightly packed RGBA.
//!
//! No dependencies, so the tests run natively:
//! `rustc --edition 2021 --test hosts/vita/src/dds.rs -o /tmp/t && /tmp/t`.

/// "DDS " and the 124-byte header.
const HEADER: usize = 128;
/// Pixel format flags.
const DDPF_FOURCC: u32 = 0x4;
const DDPF_RGB: u32 = 0x40;
const DDPF_ALPHAPIXELS: u32 = 0x1;
/// Largest side accepted; Vita textures are at most 4096.
const SIDE_MAX: u32 = 4096;

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// Whether the bytes start like a DDS file.
pub fn is_dds(bytes: &[u8]) -> bool {
    bytes.starts_with(b"DDS ")
}

/// Width, height and RGBA pixels of the first level, or None for a file this
/// does not read (a format other than the ones above, or a short file).
pub fn decode(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if !is_dds(bytes) || u32_at(bytes, 4)? != 124 {
        return None;
    }
    let height = u32_at(bytes, 12)?;
    let width = u32_at(bytes, 16)?;
    if width == 0 || height == 0 || width > SIDE_MAX || height > SIDE_MAX {
        return None;
    }
    let flags = u32_at(bytes, 80)?;
    let data = bytes.get(HEADER..)?;
    if flags & DDPF_FOURCC != 0 {
        let block: fn(&[u8], &mut [[u8; 4]; 16]) = match bytes.get(84..88)? {
            b"DXT1" => dxt1_block,
            b"DXT3" => dxt3_block,
            b"DXT5" => dxt5_block,
            _ => return None,
        };
        let size = if bytes.get(84..88)? == b"DXT1" { 8 } else { 16 };
        return Some((width, height, blocks(data, width, height, size, block)?));
    }
    if flags & DDPF_RGB != 0 && u32_at(bytes, 88)? == 32 {
        let masks = [u32_at(bytes, 92)?, u32_at(bytes, 96)?, u32_at(bytes, 100)?, u32_at(bytes, 104)?];
        let alpha = flags & DDPF_ALPHAPIXELS != 0;
        return Some((width, height, uncompressed(data, width, height, masks, alpha)?));
    }
    None
}

/// Decode a block-compressed image: 4x4 blocks, left to right, top to bottom.
fn blocks(
    data: &[u8],
    width: u32,
    height: u32,
    size: usize,
    block: fn(&[u8], &mut [[u8; 4]; 16]),
) -> Option<Vec<u8>> {
    let (across, down) = (width.div_ceil(4) as usize, height.div_ceil(4) as usize);
    let data = data.get(..across * down * size)?;
    let (width, height) = (width as usize, height as usize);
    let mut rgba = vec![0u8; width * height * 4];
    let mut texels = [[0u8; 4]; 16];
    for (index, bytes) in data.chunks_exact(size).enumerate() {
        block(bytes, &mut texels);
        let (bx, by) = (index % across * 4, index / across * 4);
        for (texel, color) in texels.iter().enumerate() {
            let (x, y) = (bx + texel % 4, by + texel / 4);
            if x < width && y < height {
                let at = (y * width + x) * 4;
                rgba[at..at + 4].copy_from_slice(color);
            }
        }
    }
    Some(rgba)
}

/// An RGB565 color as 8-bit channels.
fn rgb565(value: u16) -> [u8; 3] {
    let r = (value >> 11) & 0x1f;
    let g = (value >> 5) & 0x3f;
    let b = value & 0x1f;
    [(r * 255 / 31) as u8, (g * 255 / 63) as u8, (b * 255 / 31) as u8]
}

/// The color part of a block: two RGB565 end points and a 2-bit index per
/// texel. `transparent_black` is DXT1's mode where the first end point is not
/// above the second: index 3 is then transparent.
fn color_block(bytes: &[u8], texels: &mut [[u8; 4]; 16], transparent_black: bool) {
    let c0 = u16::from_le_bytes([bytes[0], bytes[1]]);
    let c1 = u16::from_le_bytes([bytes[2], bytes[3]]);
    let (a, b) = (rgb565(c0), rgb565(c1));
    let mix = |wa: u16, wb: u16, total: u16| -> [u8; 4] {
        let channel = |i: usize| ((a[i] as u16 * wa + b[i] as u16 * wb) / total) as u8;
        [channel(0), channel(1), channel(2), 255]
    };
    let palette = if c0 > c1 || !transparent_black {
        [mix(1, 0, 1), mix(0, 1, 1), mix(2, 1, 3), mix(1, 2, 3)]
    } else {
        [mix(1, 0, 1), mix(0, 1, 1), mix(1, 1, 2), [0, 0, 0, 0]]
    };
    let indices = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    for (texel, color) in texels.iter_mut().enumerate() {
        *color = palette[((indices >> (texel * 2)) & 3) as usize];
    }
}

fn dxt1_block(bytes: &[u8], texels: &mut [[u8; 4]; 16]) {
    color_block(bytes, texels, true);
}

/// DXT3: 4-bit alpha per texel, then a DXT1 color block.
fn dxt3_block(bytes: &[u8], texels: &mut [[u8; 4]; 16]) {
    color_block(&bytes[8..], texels, false);
    for (texel, color) in texels.iter_mut().enumerate() {
        let nibble = (bytes[texel / 2] >> ((texel % 2) * 4)) & 0xf;
        color[3] = nibble * 17;
    }
}

/// DXT5: two alpha end points and a 3-bit index per texel, then a DXT1 color block.
fn dxt5_block(bytes: &[u8], texels: &mut [[u8; 4]; 16]) {
    color_block(&bytes[8..], texels, false);
    let (a0, a1) = (bytes[0] as u16, bytes[1] as u16);
    let mut alphas = [a0, a1, 0, 0, 0, 0, 0, 0];
    if a0 > a1 {
        for i in 1..7 {
            alphas[i + 1] = ((7 - i) as u16 * a0 + i as u16 * a1) / 7;
        }
    } else {
        for i in 1..5 {
            alphas[i + 1] = ((5 - i) as u16 * a0 + i as u16 * a1) / 5;
        }
        alphas[6] = 0;
        alphas[7] = 255;
    }
    let mut bits = 0u64;
    for (i, &byte) in bytes[2..8].iter().enumerate() {
        bits |= (byte as u64) << (8 * i);
    }
    for (texel, color) in texels.iter_mut().enumerate() {
        color[3] = alphas[((bits >> (texel * 3)) & 7) as usize] as u8;
    }
}

/// A 32-bit image with channel masks (A8R8G8B8 and its orders).
fn uncompressed(data: &[u8], width: u32, height: u32, masks: [u32; 4], alpha: bool) -> Option<Vec<u8>> {
    let count = width as usize * height as usize;
    let data = data.get(..count * 4)?;
    let channel = |value: u32, mask: u32| -> u8 {
        if mask == 0 {
            return 0;
        }
        let shifted = (value & mask) >> mask.trailing_zeros();
        let max = mask >> mask.trailing_zeros();
        (shifted * 255 / max) as u8
    };
    let mut rgba = Vec::with_capacity(count * 4);
    for pixel in data.chunks_exact(4) {
        let value = u32::from_le_bytes([pixel[0], pixel[1], pixel[2], pixel[3]]);
        let a = if alpha { channel(value, masks[3]) } else { 255 };
        rgba.extend_from_slice(&[channel(value, masks[0]), channel(value, masks[1]), channel(value, masks[2]), a]);
    }
    Some(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A DDS header for a `width` x `height` image with this pixel format.
    fn header(width: u32, height: u32, four_cc: Option<&[u8; 4]>) -> Vec<u8> {
        let mut bytes = vec![0u8; HEADER];
        bytes[..4].copy_from_slice(b"DDS ");
        bytes[4..8].copy_from_slice(&124u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&height.to_le_bytes());
        bytes[16..20].copy_from_slice(&width.to_le_bytes());
        bytes[76..80].copy_from_slice(&32u32.to_le_bytes());
        match four_cc {
            Some(code) => {
                bytes[80..84].copy_from_slice(&DDPF_FOURCC.to_le_bytes());
                bytes[84..88].copy_from_slice(code);
            }
            None => {
                bytes[80..84].copy_from_slice(&(DDPF_RGB | DDPF_ALPHAPIXELS).to_le_bytes());
                bytes[88..92].copy_from_slice(&32u32.to_le_bytes());
                bytes[92..96].copy_from_slice(&0x00ff_0000u32.to_le_bytes());
                bytes[96..100].copy_from_slice(&0x0000_ff00u32.to_le_bytes());
                bytes[100..104].copy_from_slice(&0x0000_00ffu32.to_le_bytes());
                bytes[104..108].copy_from_slice(&0xff00_0000u32.to_le_bytes());
            }
        }
        bytes
    }

    #[test]
    fn dxt1_end_points_and_blend() {
        // Red (0xF800) and blue (0x001F); texels take index 0, 1, 2, 3 in turn.
        let mut file = header(4, 4, Some(b"DXT1"));
        file.extend_from_slice(&[0x00, 0xf8, 0x1f, 0x00, 0xe4, 0xe4, 0xe4, 0xe4]);
        let (width, height, rgba) = decode(&file).unwrap();
        assert_eq!((width, height), (4, 4));
        assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
        assert_eq!(&rgba[4..8], &[0, 0, 255, 255]);
        assert_eq!(&rgba[8..12], &[170, 0, 85, 255]);
        assert_eq!(&rgba[12..16], &[85, 0, 170, 255]);
    }

    #[test]
    fn dxt1_transparent_mode() {
        // First end point below the second: index 3 is transparent.
        let mut file = header(4, 4, Some(b"DXT1"));
        file.extend_from_slice(&[0x1f, 0x00, 0x00, 0xf8, 0xff, 0xff, 0xff, 0xff]);
        let (_, _, rgba) = decode(&file).unwrap();
        assert_eq!(&rgba[0..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn dxt1_places_blocks_left_to_right() {
        // An 8x4 image: a white block, then a black one.
        let mut file = header(8, 4, Some(b"DXT1"));
        file.extend_from_slice(&[0xff, 0xff, 0x00, 0x00, 0, 0, 0, 0]);
        file.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0, 0, 0, 0]);
        let (_, _, rgba) = decode(&file).unwrap();
        assert_eq!(&rgba[0..4], &[255, 255, 255, 255]);
        assert_eq!(&rgba[4 * 4..4 * 4 + 4], &[0, 0, 0, 255]);
    }

    #[test]
    fn dxt5_alpha() {
        let mut file = header(4, 4, Some(b"DXT5"));
        // Alpha end points 255 and 0, every texel index 1 (alpha 0); white color.
        file.extend_from_slice(&[255, 0, 0b0100_1001, 0b1001_0010, 0b0010_0100, 0b0100_1001, 0b1001_0010, 0b0010_0100]);
        file.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0]);
        let (_, _, rgba) = decode(&file).unwrap();
        assert_eq!(&rgba[0..4], &[255, 255, 255, 0]);
    }

    #[test]
    fn uncompressed_argb() {
        let mut file = header(1, 1, None);
        file.extend_from_slice(&0x8011_2233u32.to_le_bytes());
        let (_, _, rgba) = decode(&file).unwrap();
        assert_eq!(rgba, vec![0x11, 0x22, 0x33, 0x80]);
    }

    #[test]
    fn rejects_short_and_unknown() {
        let mut file = header(4, 4, Some(b"DXT1"));
        assert!(decode(&file).is_none());
        file[84..88].copy_from_slice(b"ATI2");
        file.extend_from_slice(&[0; 16]);
        assert!(decode(&file).is_none());
        assert!(decode(b"PNG").is_none());
    }
}
