//! A software stand-in for the vita2d backend: the same two primitives,
//! sampled the way the GPU does (pixel centers, straight-alpha blending,
//! bilinear or nearest texels, A8 textures read as white with alpha).

use shelf_core::gfx::{Cmd, Color, Format, Rect, Texture, Textures};

pub struct Canvas {
    pub w: u32,
    pub h: u32,
    /// RGB, opaque.
    pub pixels: Vec<[f32; 3]>,
}

impl Canvas {
    pub fn new(w: u32, h: u32) -> Self {
        Canvas { w, h, pixels: vec![[0.0; 3]; (w * h) as usize] }
    }

    pub fn run(&mut self, cmds: &[Cmd], textures: &Textures) {
        self.pixels.fill([0.0; 3]);
        // The core cuts every rectangle to its clip; the canvas only keeps
        // to its own edges.
        let clip = Rect::new(0.0, 0.0, self.w as f32, self.h as f32);
        for cmd in cmds {
            match *cmd {
                Cmd::Fill { rect, color } => self.each(rect, clip, |_, _| tint([1.0; 4], color)),
                Cmd::Image { tex, src, dst, tint: color } => {
                    let texture = textures.get(tex);
                    let (sx, sy) = (src.w / dst.w, src.h / dst.h);
                    self.each(dst, clip, |cx, cy| {
                        let u = src.x + (cx - dst.x) * sx;
                        let v = src.y + (cy - dst.y) * sy;
                        tint(sample(texture, u, v), color)
                    });
                }
            }
        }
    }

    /// Blend `shade(center_x, center_y)` into every pixel whose center is in
    /// both rectangles.
    fn each(&mut self, rect: Rect, clip: Rect, shade: impl Fn(f32, f32) -> [f32; 4]) {
        let x0 = rect.x.max(clip.x).max(0.0);
        let y0 = rect.y.max(clip.y).max(0.0);
        let x1 = (rect.x + rect.w).min(clip.x + clip.w).min(self.w as f32);
        let y1 = (rect.y + rect.h).min(clip.y + clip.h).min(self.h as f32);
        // Pixel i covers [i, i+1); it is drawn when its center i+0.5 is inside.
        let (px0, py0) = ((x0 - 0.5).ceil().max(0.0) as u32, (y0 - 0.5).ceil().max(0.0) as u32);
        let (px1, py1) = ((x1 - 0.5).ceil().max(0.0) as u32, (y1 - 0.5).ceil().max(0.0) as u32);
        for py in py0..py1.min(self.h) {
            for px in px0..px1.min(self.w) {
                let [r, g, b, a] = shade(px as f32 + 0.5, py as f32 + 0.5);
                if a <= 0.0 {
                    continue;
                }
                let dst = &mut self.pixels[(py * self.w + px) as usize];
                dst[0] = r * a + dst[0] * (1.0 - a);
                dst[1] = g * a + dst[1] * (1.0 - a);
                dst[2] = b * a + dst[2] * (1.0 - a);
            }
        }
    }

    pub fn save_png(&self, path: &std::path::Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), self.w, self.h);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(std::io::Error::other)?;
        let bytes: Vec<u8> = self
            .pixels
            .iter()
            .flat_map(|p| p.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8))
            .collect();
        writer.write_image_data(&bytes).map_err(std::io::Error::other)
    }
}

fn tint(texel: [f32; 4], color: Color) -> [f32; 4] {
    [
        texel[0] * color.r as f32 / 255.0,
        texel[1] * color.g as f32 / 255.0,
        texel[2] * color.b as f32 / 255.0,
        texel[3] * color.a as f32 / 255.0,
    ]
}

fn texel(texture: &Texture, x: i32, y: i32) -> [f32; 4] {
    let x = x.clamp(0, texture.w as i32 - 1) as usize;
    let y = y.clamp(0, texture.h as i32 - 1) as usize;
    let at = y * texture.w as usize + x;
    match texture.format {
        Format::Rgba8 => {
            let p = &texture.pixels[at * 4..at * 4 + 4];
            [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0, p[3] as f32 / 255.0]
        }
        Format::A8 => [1.0, 1.0, 1.0, texture.pixels[at] as f32 / 255.0],
    }
}

fn sample(texture: &Texture, u: f32, v: f32) -> [f32; 4] {
    if !texture.linear {
        return texel(texture, u.floor() as i32, v.floor() as i32);
    }
    let (fx, fy) = (u - 0.5, v - 0.5);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (wx, wy) = (fx - x0, fy - y0);
    let (x0, y0) = (x0 as i32, y0 as i32);
    let corners = [
        (texel(texture, x0, y0), (1.0 - wx) * (1.0 - wy)),
        (texel(texture, x0 + 1, y0), wx * (1.0 - wy)),
        (texel(texture, x0, y0 + 1), (1.0 - wx) * wy),
        (texel(texture, x0 + 1, y0 + 1), wx * wy),
    ];
    let mut out = [0.0; 4];
    for (texel, weight) in corners {
        for c in 0..4 {
            out[c] += texel[c] * weight;
        }
    }
    out
}
