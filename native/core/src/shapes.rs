//! Shapes built from the two primitives. A shape's coverage is computed once
//! on the CPU into a one-byte texture, cached, and drawn tinted.

use crate::gfx::{Color, Format, Gfx, Rect, TexId};

/// Coverage of a pixel whose center is `distance` from a shape's edge
/// (negative inside). `soft` 1 is a one-pixel ramp (an antialiased edge);
/// larger values fade over that many pixels inward from the edge (shadows).
fn coverage(distance: f32, soft: f32) -> u8 {
    let c = if soft <= 1.0 { 0.5 - distance } else { -distance / soft };
    (c.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn mask(size: u32, distance: impl Fn(f32, f32) -> f32, soft: f32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            pixels.push(coverage(distance(x as f32 + 0.5, y as f32 + 0.5), soft));
        }
    }
    pixels
}

fn length(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

/// The PlayStation face-button symbols.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Symbol {
    Circle,
    Cross,
    Square,
    Triangle,
}

impl Gfx {
    fn cached(&mut self, key: u64, make: impl FnOnce() -> (u32, Vec<u8>)) -> TexId {
        if let Some(&id) = self.shapes.get(&key) {
            return id;
        }
        let (size, pixels) = make();
        let id = self.add_texture(size, size, Format::A8, pixels, false);
        self.shapes.insert(key, id);
        id
    }

    /// A circle of `radius` pixels in a `2·radius` square: filled, or a ring
    /// `thickness` wide, its edge faded over `soft` pixels. Its quarters are
    /// the corners of a rounded rectangle, its middle row and column the edges.
    fn corner_texture(&mut self, radius: u32, thickness: u32, soft: u32) -> TexId {
        let key = (1u64 << 56) | ((radius as u64) << 32) | ((thickness as u64) << 16) | soft as u64;
        self.cached(key, || {
            let (r, t) = (radius as f32, thickness as f32);
            let size = radius * 2;
            let pixels = mask(
                size,
                |x, y| {
                    let d = length(x - r, y - r) - r;
                    if thickness == 0 { d } else { d.max(-(d + t)) }
                },
                soft as f32,
            );
            (size, pixels)
        })
    }

    /// Nine pieces of a corner texture over a pixel rectangle.
    fn nine(&mut self, rect: Rect, r: f32, tex: TexId, filled: bool, color: Color) {
        let Rect { x, y, w, h } = rect;
        let (iw, ih) = (w - 2.0 * r, h - 2.0 * r);
        // Corners.
        self.raw_image(tex, Rect::new(0.0, 0.0, r, r), Rect::new(x, y, r, r), color);
        self.raw_image(tex, Rect::new(r, 0.0, r, r), Rect::new(x + w - r, y, r, r), color);
        self.raw_image(tex, Rect::new(0.0, r, r, r), Rect::new(x, y + h - r, r, r), color);
        self.raw_image(tex, Rect::new(r, r, r, r), Rect::new(x + w - r, y + h - r, r, r), color);
        // Edges: one texel column or row next to the middle, stretched.
        self.raw_image(tex, Rect::new(r - 1.0, 0.0, 1.0, r), Rect::new(x + r, y, iw, r), color);
        self.raw_image(tex, Rect::new(r - 1.0, r, 1.0, r), Rect::new(x + r, y + h - r, iw, r), color);
        self.raw_image(tex, Rect::new(0.0, r - 1.0, r, 1.0), Rect::new(x, y + r, r, ih), color);
        self.raw_image(tex, Rect::new(r, r - 1.0, r, 1.0), Rect::new(x + w - r, y + r, r, ih), color);
        if filled {
            self.raw_fill(Rect::new(x + r, y + r, iw, ih), color);
        }
    }

    /// A rectangle with rounded corners, filled (`thickness` 0) or as a
    /// ring. Logical units; snapped to whole screen pixels.
    pub fn round_rect(&mut self, rect: Rect, radius: f32, thickness: f32, color: Color) {
        let s = self.scale;
        let px = rect.scale(s).round();
        if px.w <= 0.0 || px.h <= 0.0 {
            return;
        }
        let r = (radius * s).round().min((px.w / 2.0).floor()).min((px.h / 2.0).floor()).max(1.0);
        let t = (thickness * s).round();
        let tex = self.corner_texture(r as u32, t as u32, 1);
        self.nine(px, r, tex, t == 0.0, color);
    }

    /// A soft shadow under `rect`: dark at its edge, fading out over `blur`
    /// units around it (the current UI's `shadow-lg`).
    pub fn shadow(&mut self, rect: Rect, radius: f32, blur: f32, color: Color) {
        let s = self.scale;
        let px = rect.grow(blur).scale(s).round();
        let soft = (2.0 * blur * s).round().max(2.0);
        let r = ((radius + blur) * s).round().min((px.w / 2.0).floor()).min((px.h / 2.0).floor());
        let tex = self.corner_texture(r as u32, 0, soft as u32);
        self.nine(px, r, tex, true, color);
    }

    /// A face-button symbol `size` units square, top-left at `(x, y)`.
    pub fn symbol(&mut self, symbol: Symbol, x: f32, y: f32, size: f32, color: Color) {
        let px = (size * self.scale).round() as u32;
        let key = (2u64 << 56) | ((px as u64) << 8) | symbol as u64;
        let tex = self.cached(key, || {
            let c = px as f32 / 2.0;
            let unit = px as f32 / 22.0;
            let half = 1.1 * unit;
            let pixels = mask(
                px,
                |x, y| {
                    let (px, py) = ((x - c) / unit, (y - c) / unit);
                    let d = match symbol {
                        Symbol::Circle => (length(px, py) - 7.5).abs(),
                        Symbol::Square => {
                            let (dx, dy) = (px.abs() - 7.0, py.abs() - 7.0);
                            (length(dx.max(0.0), dy.max(0.0)) + dx.max(dy).min(0.0)).abs()
                        }
                        Symbol::Cross => {
                            let arm = 6.5;
                            let segment = |sx: f32| {
                                let (ax, ay) = (px + arm, py + sx * arm);
                                let (bx, by) = (2.0 * arm, -2.0 * sx * arm);
                                let h = ((ax * bx + ay * by) / (bx * bx + by * by)).clamp(0.0, 1.0);
                                length(ax - bx * h, ay - by * h)
                            };
                            segment(1.0).min(segment(-1.0))
                        }
                        Symbol::Triangle => {
                            // Equilateral, pointing up (y grows downward).
                            let r = 7.5;
                            let k = 3f32.sqrt();
                            let mut qx = px.abs() - r;
                            let mut qy = -(py - 1.5) + r / k;
                            if qx + k * qy > 0.0 {
                                let (nx, ny) = ((qx - k * qy) / 2.0, (-k * qx - qy) / 2.0);
                                qx = nx;
                                qy = ny;
                            }
                            qx -= qx.clamp(-2.0 * r, 0.0);
                            (-length(qx, qy) * qy.signum()).abs()
                        }
                    };
                    d * unit - half
                },
                1.0,
            );
            (px, pixels)
        });
        let s = self.scale;
        let dst = Rect::new((x * s).round(), (y * s).round(), px as f32, px as f32);
        self.raw_image(tex, Rect::new(0.0, 0.0, px as f32, px as f32), dst, color);
    }

    /// A vertical gradient texture (8×256, bilinear), stretched by the caller.
    pub fn gradient(&mut self, top: Color, bottom: Color) -> TexId {
        let mut pixels = Vec::with_capacity(8 * 256 * 4);
        for y in 0..256 {
            let c = top.lerp(bottom, y as f32 / 255.0);
            for _ in 0..8 {
                pixels.extend_from_slice(&[c.r, c.g, c.b, c.a]);
            }
        }
        self.add_texture(8, 256, Format::Rgba8, pixels, true)
    }
}

/// Make the corners of an RGBA picture transparent, antialiased: rounded
/// artwork without a clipping stage at draw time.
pub fn round_corners(rgba: &mut [u8], w: u32, h: u32, radius: f32) {
    let r = radius.min(w as f32 / 2.0).min(h as f32 / 2.0);
    if r <= 0.0 {
        return;
    }
    for y in 0..h {
        for x in 0..w {
            let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
            let dx = (r - cx).max(cx - (w as f32 - r)).max(0.0);
            let dy = (r - cy).max(cy - (h as f32 - r)).max(0.0);
            if dx == 0.0 || dy == 0.0 {
                continue;
            }
            let keep = coverage(length(dx, dy) - r, 1.0) as u32;
            let alpha = &mut rgba[((y * w + x) * 4 + 3) as usize];
            *alpha = (*alpha as u32 * keep / 255) as u8;
        }
    }
}
