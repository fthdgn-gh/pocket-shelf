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
    /// ring; snapped to whole pixels.
    pub fn round_rect(&mut self, rect: Rect, radius: f32, thickness: f32, color: Color) {
        let px = rect.round();
        if px.w <= 0.0 || px.h <= 0.0 {
            return;
        }
        let r = radius.round().min((px.w / 2.0).floor()).min((px.h / 2.0).floor()).max(1.0);
        let t = thickness.round();
        let tex = self.corner_texture(r as u32, t as u32, 1);
        self.nine(px, r, tex, t == 0.0, color);
    }

    /// A soft shadow under `rect`: dark at its edge, fading out over `blur`
    /// pixels around it (the current UI's `shadow-lg`).
    pub fn shadow(&mut self, rect: Rect, radius: f32, blur: f32, color: Color) {
        let px = rect.grow(blur).round();
        let soft = (2.0 * blur).round().max(2.0);
        let r = (radius + blur).round().min((px.w / 2.0).floor()).min((px.h / 2.0).floor());
        let tex = self.corner_texture(r as u32, 0, soft as u32);
        self.nine(px, r, tex, true, color);
    }

    /// A face-button symbol `size` pixels square, top-left at `(x, y)`.
    pub fn symbol(&mut self, symbol: Symbol, x: f32, y: f32, size: f32, color: Color) {
        let px = size.round() as u32;
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
                                // From (-arm, -sx·arm) to (arm, sx·arm): one diagonal.
                                let (ax, ay) = (px + arm, py + sx * arm);
                                let (bx, by) = (2.0 * arm, 2.0 * sx * arm);
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
        let dst = Rect::new(x.round(), y.round(), px as f32, px as f32);
        self.raw_image(tex, Rect::new(0.0, 0.0, px as f32, px as f32), dst, color);
    }

    /// An icon drawn from its outline, with its top left at `(x, y)`,
    /// tinted.
    pub fn icon(&mut self, icon: Icon, x: f32, y: f32, color: Color) {
        let (w, h) = icon.size();
        let (pw, ph) = (w.round() as u32, h.round() as u32);
        let key = (3u64 << 56) | ((pw as u64) << 32) | icon.key();
        let tex = match self.shapes.get(&key) {
            Some(&id) => id,
            None => {
                let mut pixels = Vec::with_capacity((pw * ph) as usize);
                for py in 0..ph {
                    for px in 0..pw {
                        let d = icon.distance(px as f32 + 0.5, py as f32 + 0.5);
                        pixels.push(coverage(d, 1.0));
                    }
                }
                let id = self.add_texture(pw, ph, Format::A8, pixels, false);
                self.shapes.insert(key, id);
                id
            }
        };
        let dst = Rect::new(x.round(), y.round(), pw as f32, ph as f32);
        self.raw_image(tex, Rect::new(0.0, 0.0, pw as f32, ph as f32), dst, color);
    }

    /// A face button as Pocket Shelf 0.1's prompts draw it: the symbol in its
    /// color on a dark disc, 32 pixels square.
    pub fn button(&mut self, symbol: Symbol, x: f32, y: f32) {
        self.round_rect(Rect::new(x + 1.0, y + 1.0, 30.0, 30.0), 15.0, 0.0, Color::rgb(0x3d4350));
        let color = match symbol {
            Symbol::Circle => 0xfb7185,
            Symbol::Cross => 0x93c5fd,
            Symbol::Triangle => 0x5eead4,
            Symbol::Square => 0xf0abfc,
        };
        self.symbol(symbol, x + 6.0, y + 6.0, 20.0, Color::rgb(color));
    }

    /// A vertical gradient texture (8×256, bilinear), stretched by the caller.
    pub fn gradient(&mut self, top: Color, bottom: Color) -> TexId {
        self.gradient_sized(top, bottom, 256)
    }

    /// A vertical gradient `steps` texels high (8 wide), for small areas.
    pub fn gradient_sized(&mut self, top: Color, bottom: Color, steps: u32) -> TexId {
        let mut pixels = Vec::with_capacity((8 * steps * 4) as usize);
        for y in 0..steps {
            let c = top.lerp(bottom, y as f32 / (steps - 1) as f32);
            for _ in 0..8 {
                pixels.extend_from_slice(&[c.r, c.g, c.b, c.a]);
            }
        }
        self.add_texture(8, steps, Format::Rgba8, pixels, true)
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

/// Icons outlined from Pocket Shelf 0.1's SVGs at twice their size (the
/// SVGs were drawn for a 480×272 layout), in pixels, drawn in one tint each.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    /// The L shoulder button: a body with its outer top corner swept, 48×32.
    ShoulderL,
    ShoulderR,
    /// The Wi-Fi rings lit for a level (0 to 3), or (`lit` false) the rest:
    /// a dot and three quarter rings, 24×24 (the Vita's own symbol).
    Wifi { level: u8, lit: bool },
    /// The Bluetooth rune, 24×24.
    Bluetooth,
    /// A small triangle pointing right, 16×32: a drawer row that opens
    /// something or steps a value.
    ArrowRight,
    ArrowLeft,
}

/// Distance from `(x, y)` to the segment `a`–`b`.
fn segment(x: f32, y: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (px, py, bx, by) = (x - a.0, y - a.1, b.0 - a.0, b.1 - a.1);
    let h = ((px * bx + py * by) / (bx * bx + by * by)).clamp(0.0, 1.0);
    length(px - bx * h, py - by * h)
}

/// Signed distance to a box from `(x0, y0)` to `(x1, y1)` with corner radii
/// top left, top right, bottom right, bottom left.
fn round_box(x: f32, y: f32, (x0, y0, x1, y1): (f32, f32, f32, f32), radii: [f32; 4]) -> f32 {
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (hw, hh) = ((x1 - x0) / 2.0, (y1 - y0) / 2.0);
    let (dx, dy) = (x - cx, y - cy);
    let r = match (dx < 0.0, dy < 0.0) {
        (true, true) => radii[0],
        (false, true) => radii[1],
        (false, false) => radii[2],
        (true, false) => radii[3],
    };
    let (qx, qy) = (dx.abs() - hw + r, dy.abs() - hh + r);
    length(qx.max(0.0), qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

impl Icon {
    fn size(self) -> (f32, f32) {
        match self {
            Icon::ShoulderL | Icon::ShoulderR => (48.0, 32.0),
            Icon::Wifi { .. } | Icon::Bluetooth => (24.0, 24.0),
            Icon::ArrowRight | Icon::ArrowLeft => (16.0, 32.0),
        }
    }

    fn key(self) -> u64 {
        match self {
            Icon::ShoulderL => 1,
            Icon::ShoulderR => 2,
            Icon::Wifi { level, lit } => 16 + level as u64 * 2 + lit as u64,
            Icon::Bluetooth => 3,
            Icon::ArrowRight => 4,
            Icon::ArrowLeft => 5,
        }
    }

    /// Signed distance in pixels (negative inside).
    fn distance(self, x: f32, y: f32) -> f32 {
        match self {
            // `btn-l.svg` at 2×: radius 6 corners, the outer top corner a
            // curve from (22, 4) down to (0, 22), here a radius of 18.
            Icon::ShoulderL => round_box(x, y, (0.0, 4.0, 48.0, 28.0), [18.0, 6.0, 6.0, 6.0]),
            Icon::ShoulderR => round_box(x, y, (0.0, 4.0, 48.0, 28.0), [6.0, 18.0, 6.0, 6.0]),
            Icon::Wifi { level, lit } => {
                // Centered on (4, 20): the dot, then rings 2.8 wide.
                let r = length(x - 4.0, y - 20.0);
                let mut d = f32::MAX;
                if lit {
                    d = r - 3.4;
                }
                for (ring, mid) in [8.2f32, 13.2, 18.2].into_iter().enumerate() {
                    if ((ring as u8) < level) == lit {
                        let band = (r - mid).abs() - 1.4;
                        d = d.min(band.max(4.0 - x).max(y - 20.0));
                    }
                }
                d
            }
            // `arrow-right.svg` at 2×: (3, 8), (13, 16), (3, 24).
            Icon::ArrowRight | Icon::ArrowLeft => {
                let x = if self == Icon::ArrowLeft { 16.0 - x } else { x };
                let (dx, dy) = (x - 3.0, (y - 16.0).abs());
                // Inside: right of the back edge and under the slanted one.
                let slant = (dy * 5.0 + (x - 13.0) * 4.0) / (41f32).sqrt();
                (-dx).max(slant)
            }
            Icon::Bluetooth => {
                let lines = [
                    ((12.0, 2.0), (12.0, 22.0)),
                    ((12.0, 2.0), (17.4, 7.2)),
                    ((17.4, 7.2), (6.6, 16.8)),
                    ((12.0, 22.0), (17.4, 16.8)),
                    ((17.4, 16.8), (6.6, 7.2)),
                ];
                lines.iter().map(|&(a, b)| segment(x, y, a, b)).fold(f32::MAX, f32::min) - 1.2
            }
        }
    }
}
