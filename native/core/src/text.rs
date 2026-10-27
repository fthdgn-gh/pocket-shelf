//! Text: Figtree rasterized by `fontdue` into one shared glyph atlas, each
//! glyph the first time it is drawn at a size.
//!
//! Sizes are logical (the current UI's `text-*` classes); glyphs are
//! rasterized at `size × scale` screen pixels.

use std::collections::HashMap;

use fontdue::{Font, FontSettings};

use crate::gfx::{Color, Format, Gfx, Rect, TexId, Textures};

const REGULAR: &[u8] = include_bytes!("../../../src/fonts/Figtree-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../../src/fonts/Figtree-Bold.ttf");

const ATLAS: u32 = 1024;
const PAD: u32 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Weight {
    Regular,
    Bold,
}

/// Where a piece of text is used; each role has one size and weight
/// (`src/text.ts`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// 24, bold.
    Display,
    /// 20, bold.
    Heading,
    /// 16, bold.
    Title,
    /// 14.
    Body,
    /// 14, bold.
    BodyBold,
    /// 12.
    Small,
    /// 12, bold.
    SmallBold,
    /// 12, bold, letters 0.025 em apart (`tracking-wide`).
    Label,
}

impl Role {
    pub fn style(self) -> (Weight, f32, f32) {
        match self {
            Role::Display => (Weight::Bold, 24.0, 0.0),
            Role::Heading => (Weight::Bold, 20.0, 0.0),
            Role::Title => (Weight::Bold, 16.0, 0.0),
            Role::Body => (Weight::Regular, 14.0, 0.0),
            Role::BodyBold => (Weight::Bold, 14.0, 0.0),
            Role::Small => (Weight::Regular, 12.0, 0.0),
            Role::SmallBold => (Weight::Bold, 12.0, 0.0),
            Role::Label => (Weight::Bold, 12.0, 0.3),
        }
    }
}

#[derive(Clone, Copy)]
struct Glyph {
    /// Place in the atlas; `None` for a glyph with no pixels (a space).
    src: Option<Rect>,
    /// Offset of the bitmap's top-left from the pen on the baseline, pixels.
    left: f32,
    top: f32,
    advance: f32,
}

pub(crate) struct FontState {
    regular: Font,
    bold: Font,
    glyphs: HashMap<(Weight, u16, char), Glyph>,
    atlas: TexId,
    pen_x: u32,
    pen_y: u32,
    row_h: u32,
}

impl FontState {
    pub(crate) fn new(textures: &mut Textures) -> Self {
        let load = |bytes| Font::from_bytes(bytes, FontSettings::default()).expect("font");
        let atlas = textures.add(ATLAS, ATLAS, Format::A8, vec![0; (ATLAS * ATLAS) as usize], true);
        textures.list[atlas.0 as usize].keep = true;
        FontState { regular: load(REGULAR), bold: load(BOLD), glyphs: HashMap::new(), atlas, pen_x: 0, pen_y: 0, row_h: 0 }
    }

    fn font(&self, weight: Weight) -> &Font {
        match weight {
            Weight::Regular => &self.regular,
            Weight::Bold => &self.bold,
        }
    }

    /// The glyph at `px` screen pixels, and whether it was new.
    fn glyph(&mut self, textures: &mut Textures, weight: Weight, px: u16, ch: char) -> (Glyph, bool) {
        if let Some(&glyph) = self.glyphs.get(&(weight, px, ch)) {
            return (glyph, false);
        }
        let (metrics, bitmap) = self.font(weight).rasterize(ch, px as f32);
        let (w, h) = (metrics.width as u32, metrics.height as u32);
        let mut src = None;
        if w > 0 && h > 0 {
            if self.pen_x + w + PAD > ATLAS {
                self.pen_x = 0;
                self.pen_y += self.row_h + PAD;
                self.row_h = 0;
            }
            if self.pen_y + h + PAD > ATLAS {
                // Full: start over. Glyphs already drawn this frame may show
                // wrong for one frame.
                self.glyphs.clear();
                self.pen_x = 0;
                self.pen_y = 0;
                self.row_h = 0;
                textures.edit(self.atlas).pixels.fill(0);
            }
            let atlas = textures.edit_rows(self.atlas, self.pen_y, self.pen_y + h);
            for row in 0..h {
                let at = ((self.pen_y + row) * ATLAS + self.pen_x) as usize;
                atlas.pixels[at..at + w as usize].copy_from_slice(&bitmap[(row * w) as usize..((row + 1) * w) as usize]);
            }
            src = Some(Rect::new(self.pen_x as f32, self.pen_y as f32, w as f32, h as f32));
            self.pen_x += w + PAD;
            self.row_h = self.row_h.max(h);
        }
        let glyph = Glyph { src, left: metrics.xmin as f32, top: -(metrics.ymin as f32 + h as f32), advance: metrics.advance_width };
        self.glyphs.insert((weight, px, ch), glyph);
        (glyph, true)
    }

    fn kern(&self, weight: Weight, px: u16, left: char, right: char) -> f32 {
        self.font(weight).horizontal_kern(left, right, px as f32).unwrap_or(0.0)
    }

    /// Ascent and descent (negative) in pixels.
    fn line(&self, weight: Weight, px: u16) -> (f32, f32) {
        let line = self.font(weight).horizontal_line_metrics(px as f32).expect("line metrics");
        (line.ascent, line.descent)
    }
}

impl Gfx {
    fn px(&self, size: f32) -> u16 {
        (size * self.scale).round() as u16
    }

    /// Pen advance over `text` in pixels; draws it when `at` is given
    /// (pixel pen x, pixel baseline).
    fn run(&mut self, role: Role, text: &str, at: Option<(f32, f32, Color)>) -> f32 {
        let (weight, size, tracking) = role.style();
        let px = self.px(size);
        let tracking = tracking * self.scale;
        let atlas = self.fonts.atlas;
        let mut pen = 0.0;
        let mut previous = None;
        for ch in text.chars() {
            if let Some(left) = previous {
                pen += self.fonts.kern(weight, px, left, ch) + tracking;
            }
            let (glyph, new) = self.fonts.glyph(&mut self.textures, weight, px, ch);
            if new {
                self.work.glyphs += 1;
            }
            if let (Some((x, baseline, color)), Some(src)) = (at, glyph.src) {
                let dst = Rect::new((x + pen + glyph.left).round(), baseline + glyph.top, src.w, src.h);
                self.raw_image(atlas, src, dst, color);
            }
            pen += glyph.advance;
            previous = Some(ch);
        }
        pen
    }

    /// Width of `text` in logical units.
    pub fn measure(&mut self, role: Role, text: &str) -> f32 {
        self.run(role, text, None) / self.scale
    }

    /// Draw `text` with its baseline at `baseline`, starting at `x`. Returns
    /// the x after it.
    pub fn text(&mut self, role: Role, x: f32, baseline: f32, color: Color, text: &str) -> f32 {
        let s = self.scale;
        let width = self.run(role, text, Some(((x * s).round(), (baseline * s).round(), color)));
        x + width / s
    }

    /// Draw `text` with the top of its line box at `top` (the way the
    /// current UI places text in a box): the baseline is `top + ascent`.
    pub fn text_top(&mut self, role: Role, x: f32, top: f32, color: Color, text: &str) -> f32 {
        let baseline = top + self.ascent(role);
        self.text(role, x, baseline, color, text)
    }

    pub fn text_centered(&mut self, role: Role, cx: f32, baseline: f32, color: Color, text: &str) {
        let width = self.measure(role, text);
        self.text(role, cx - width / 2.0, baseline, color, text);
    }

    pub fn text_right(&mut self, role: Role, right: f32, baseline: f32, color: Color, text: &str) {
        let width = self.measure(role, text);
        self.text(role, right - width, baseline, color, text);
    }

    /// Ascent of a role's line, logical units.
    pub fn ascent(&self, role: Role) -> f32 {
        let (weight, size, _) = role.style();
        self.fonts.line(weight, self.px(size)).0 / self.scale
    }

    /// Height of a role's line (ascent + descent), logical units.
    pub fn line_height(&self, role: Role) -> f32 {
        let (weight, size, _) = role.style();
        let (ascent, descent) = self.fonts.line(weight, self.px(size));
        (ascent - descent) / self.scale
    }

    /// `text`, cut at the end with "…" to fit `max` units.
    pub fn fit(&mut self, role: Role, text: &str, max: f32) -> String {
        if self.measure(role, text) <= max {
            return text.to_string();
        }
        let mut cut: Vec<char> = text.chars().collect();
        while !cut.is_empty() {
            cut.pop();
            let candidate = cut.iter().collect::<String>().trim_end().to_string() + "…";
            if self.measure(role, &candidate) <= max {
                return candidate;
            }
        }
        "…".into()
    }

    /// The end of `text` that fits `max` units, led by "..." when it was cut
    /// (`fitTail` in `catalog.ts`, for paths).
    pub fn fit_tail(&mut self, role: Role, text: &str, max: f32) -> String {
        if self.measure(role, text) <= max {
            return text.to_string();
        }
        let chars: Vec<char> = text.chars().collect();
        let mut start = 1;
        while start + 1 < chars.len() && self.measure(role, &format!("...{}", chars[start..].iter().collect::<String>())) > max {
            start += 1;
        }
        format!("...{}", chars[start..].iter().collect::<String>())
    }

    /// `text` broken into lines at spaces so each fits `max` units; a word
    /// longer than a line keeps a line of its own (`wrapText` in `catalog.ts`).
    pub fn wrap(&mut self, role: Role, text: &str, max: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in text.split(' ') {
            let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if !line.is_empty() && self.measure(role, &next) > max {
                lines.push(std::mem::replace(&mut line, word.to_string()));
            } else {
                line = next;
            }
        }
        lines.push(line);
        lines
    }
}

/// `text` rasterized into a one-byte mask at `px` pixels, for drawing into
/// pictures on the CPU (the desktop's sample icons). Width, height, mask.
pub fn render_mask(weight: Weight, px: f32, text: &str) -> (u32, u32, Vec<u8>) {
    let font = Font::from_bytes(
        match weight {
            Weight::Regular => REGULAR,
            Weight::Bold => BOLD,
        },
        FontSettings::default(),
    )
    .expect("font");
    let line = font.horizontal_line_metrics(px).expect("line metrics");
    let width: f32 = text.chars().map(|ch| font.metrics(ch, px).advance_width).sum();
    let (w, h) = (width.ceil() as u32 + 2, (line.ascent - line.descent).ceil() as u32 + 2);
    let mut mask = vec![0u8; (w * h) as usize];
    let mut pen = 1.0f32;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, px);
        let left = (pen + metrics.xmin as f32).round() as i32;
        let top = (1.0 + line.ascent - (metrics.ymin as f32 + metrics.height as f32)).round() as i32;
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let (x, y) = (left + col as i32, top + row as i32);
                if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
                    let at = (y as u32 * w + x as u32) as usize;
                    mask[at] = mask[at].max(bitmap[row * metrics.width + col]);
                }
            }
        }
        pen += metrics.advance_width;
    }
    (w, h, mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_keeps_words_whole() {
        let mut gfx = Gfx::new(2.0);
        let lines = gfx.wrap(Role::Body, "one two three four five six seven", 60.0);
        assert!(lines.len() > 1);
        assert_eq!(lines.join(" "), "one two three four five six seven");
        for line in &lines[..lines.len() - 1] {
            assert!(gfx.measure(Role::Body, line) <= 60.0);
        }
        let long = gfx.wrap(Role::Body, "Supercalifragilistic", 10.0);
        assert_eq!(long, vec!["Supercalifragilistic"]);
    }

    #[test]
    fn fits_cut_text() {
        let mut gfx = Gfx::new(2.0);
        assert_eq!(gfx.fit(Role::Body, "short", 200.0), "short");
        let cut = gfx.fit(Role::Body, "The Long Winter of the Northern Lighthouse", 100.0);
        assert!(cut.ends_with('…') && gfx.measure(Role::Body, &cut) <= 100.0);
        let tail = gfx.fit_tail(Role::Small, "ux0:/pspemu/ISO/Racing/Some Long Game.cso", 80.0);
        assert!(tail.starts_with("...") && tail.ends_with(".cso"));
    }
}
