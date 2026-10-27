//! Pocket Shelf's LiveArea artwork in `vita/package/sce_sys/`, and the
//! LiveArea layout:
//!
//!   icon0.png                         128x128  the bubble on the home screen
//!   pic0.png                          960x544  the splash the system shows
//!                                               while the app loads
//!   livearea/contents/bg.png          840x500  the LiveArea background
//!   livearea/contents/startup.png     280x158  the gate image
//!   livearea/contents/template.xml             the layout: the app's name,
//!                                               author and version as text
//!
//! The pictures hold no text but the splash's name. The system draws its
//! "Start" label over the bottom of the gate image, which hid a name drawn
//! there; the name, author and version are LiveArea text instead (from
//! `vita/Cargo.toml`), in the `psmobile` style's frames as VitaShell's
//! LiveArea uses them: the name in frame2, "by <author>" in frame3,
//! "v<version>" in frame4.
//!
//! The artwork repeats the app's Home view in small (`MARK`): a selected
//! tile with the white selection ring and smaller tiles after it. The Vita
//! accepts these files as indexed 8-bit PNG only (`png::indexed`). The files
//! are committed; a build does not run this.

use std::fs;
use std::path::Path;

use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Mask, Paint, Path as SkPath, PathBuilder, Pixmap, Point, RadialGradient, Rect, Shader,
    SpreadMode, Stroke, Transform,
};

use crate::manifest::{self, Manifest};

/// The Midnight theme (core/src/theme.rs), the app's default.
const BG_TOP: u32 = 0x101a36;
const BG_BOTTOM: u32 = 0x02040c;

/// A tile of the mark: its left, top and side in the app's logical pixels
/// (the screen is 480x272, drawn at 2x), and its gradient.
#[derive(Clone, Copy)]
struct Tile {
    x: f32,
    y: f32,
    side: f32,
    colors: (u32, u32),
}

/// The mark: the Home view in small. A selected tile with the white
/// selection ring (2 px wide, 2 px clear of it), and two smaller tiles after
/// it, aligned at the top. The scan page (core/src/app.rs) draws the same
/// mark; keep the two in step.
const MARK_WIDTH: f32 = 110.0;
const MARK_HEIGHT: f32 = 48.0;
const MARK_RING: f32 = 2.0;
const MARK_TILE: Tile = Tile { x: 4.0, y: 4.0, side: 40.0, colors: (0xa78bfa, 0x5b21b6) };
const MARK_SMALL: [Tile; 2] = [
    Tile { x: 52.0, y: 4.0, side: 26.0, colors: (0xfbbf24, 0xdc2626) },
    Tile { x: 84.0, y: 4.0, side: 26.0, colors: (0x34d399, 0x0f766e) },
];
const MARK_SMALL_OPACITY: f32 = 0.8;

/// The splash leads into the app's first screen: the mark and the name where
/// the scan page of a first start puts them.
const SPLASH_MARK_TOP: f32 = 56.0;
const SPLASH_TITLE_Y: f32 = 138.0;

const FONT: &[u8] = include_bytes!("../../fonts/Figtree-Bold.ttf");

fn color(rgb: u32, alpha: f32) -> Color {
    Color::from_rgba8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// A picture and the canvas state the drawing uses: one global alpha, as a
/// 2D canvas's `globalAlpha`.
struct Canvas {
    pixmap: Pixmap,
    alpha: f32,
}

impl Canvas {
    fn new(width: u32, height: u32) -> Canvas {
        Canvas { pixmap: Pixmap::new(width, height).unwrap(), alpha: 1.0 }
    }

    fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, shader: Shader, mask: Option<&Mask>) {
        let paint = Paint { shader, anti_alias: true, ..Paint::default() };
        if let Some(rect) = Rect::from_xywh(x, y, w, h) {
            self.pixmap.fill_rect(rect, &paint, Transform::identity(), mask);
        }
    }

    fn fill_path(&mut self, path: &SkPath, shader: Shader, mask: Option<&Mask>) {
        let paint = Paint { shader, anti_alias: true, ..Paint::default() };
        self.pixmap.fill_path(path, &paint, FillRule::Winding, Transform::identity(), mask);
    }

    fn solid(&self, rgb: u32, alpha: f32) -> Shader<'static> {
        Shader::SolidColor(color(rgb, alpha * self.alpha))
    }

    /// A vertical gradient from `top` to `bottom` over `y0..y1`.
    fn vertical(&self, y0: f32, y1: f32, top: u32, bottom: u32) -> Shader<'static> {
        let stops = vec![GradientStop::new(0.0, color(top, self.alpha)), GradientStop::new(1.0, color(bottom, self.alpha))];
        LinearGradient::new(Point::from_xy(0.0, y0), Point::from_xy(0.0, y1), stops, SpreadMode::Pad, Transform::identity()).unwrap()
    }

    fn background(&mut self) {
        let (w, h) = (self.pixmap.width() as f32, self.pixmap.height() as f32);
        let shader = self.vertical(0.0, h, BG_TOP, BG_BOTTOM);
        self.fill_rect(0.0, 0.0, w, h, shader, None);
    }

    /// A soft round glow of `rgb`, strongest at the center.
    fn glow(&mut self, x: f32, y: f32, radius: f32, rgb: u32, strength: f32) {
        let stops = vec![GradientStop::new(0.0, color(rgb, strength * self.alpha)), GradientStop::new(1.0, color(rgb, 0.0))];
        let center = Point::from_xy(x, y);
        let shader = RadialGradient::new(center, center, radius, stops, SpreadMode::Pad, Transform::identity()).unwrap();
        self.fill_rect(x - radius, y - radius, radius * 2.0, radius * 2.0, shader, None);
    }

    /// One tile: a vertical gradient, and a lighter wedge across its top
    /// corner. Sets the global alpha to `opacity`, then back to 1.
    fn tile(&mut self, x: f32, y: f32, side: f32, colors: (u32, u32), opacity: f32) {
        self.alpha = opacity;
        let shape = rounded_rect(x, y, side, side, side * 0.08);
        let shader = self.vertical(y, y + side, colors.0, colors.1);
        self.fill_path(&shape, shader, None);
        let mut clip = Mask::new(self.pixmap.width(), self.pixmap.height()).unwrap();
        clip.fill_path(&shape, FillRule::Winding, true, Transform::identity());
        let mut wedge = PathBuilder::new();
        wedge.move_to(x, y);
        wedge.line_to(x + side, y);
        wedge.line_to(x, y + side * 0.7);
        wedge.close();
        let shader = Shader::SolidColor(color(0xffffff, opacity * 0.16));
        self.fill_path(&wedge.finish().unwrap(), shader, Some(&clip));
        self.alpha = 1.0;
    }

    /// The mark with its top left at (`left`, `top`), each logical pixel
    /// `scale` picture pixels. `extra` more small tiles continue the row to
    /// the right, fading out, for pictures where it runs off the edge.
    fn mark(&mut self, left: f32, top: f32, scale: f32, extra: usize) {
        let at = |value: f32| value * scale;
        let big = MARK_TILE;
        self.glow(left + at(big.x + big.side / 2.0), top + at(big.y + big.side / 2.0), at(big.side * 1.6), 0x8b5cf6, 0.35);
        let mut smalls = MARK_SMALL.to_vec();
        for _ in 0..extra {
            let last = *smalls.last().unwrap();
            let next = MARK_SMALL[smalls.len() % MARK_SMALL.len()];
            smalls.push(Tile { x: last.x + last.side + 6.0, ..next });
        }
        for (index, item) in smalls.iter().enumerate() {
            let fade = if index < MARK_SMALL.len() { 1.0 } else { (1.0 - (index - MARK_SMALL.len() + 1) as f32 * 0.3).max(0.25) };
            self.tile(left + at(item.x), top + at(item.y), at(item.side), item.colors, MARK_SMALL_OPACITY * fade);
        }
        self.tile(left + at(big.x), top + at(big.y), at(big.side), big.colors, 1.0);
        let inset = at(MARK_RING / 2.0);
        let side = at(big.side + 8.0) - inset * 2.0;
        let ring = rounded_rect(left + inset, top + inset, side, side, at(3.0));
        let paint = Paint { shader: self.solid(0xffffff, 1.0), anti_alias: true, ..Paint::default() };
        let stroke = Stroke { width: at(MARK_RING), ..Stroke::default() };
        self.pixmap.stroke_path(&ring, &paint, &stroke, Transform::identity(), None);
    }

    /// `text` in white, centered on (`cx`, `cy`), the middle of its em box
    /// on `cy` (a canvas's `textAlign = center`, `textBaseline = middle`).
    fn text_centered(&mut self, text: &str, px: f32, cx: f32, cy: f32) {
        let font = fontdue::Font::from_bytes(FONT, fontdue::FontSettings::default()).unwrap();
        let line = font.horizontal_line_metrics(px).unwrap();
        let chars: Vec<char> = text.chars().collect();
        let mut width = 0.0;
        for (i, &c) in chars.iter().enumerate() {
            width += font.metrics(c, px).advance_width;
            if let Some(&next) = chars.get(i + 1) {
                width += font.horizontal_kern(c, next, px).unwrap_or(0.0);
            }
        }
        let baseline = cy + (line.ascent + line.descent) / 2.0;
        let mut pen = cx - width / 2.0;
        let (w, h) = (self.pixmap.width() as i32, self.pixmap.height() as i32);
        for (i, &c) in chars.iter().enumerate() {
            let (metrics, coverage) = font.rasterize(c, px);
            let left = (pen + metrics.xmin as f32).round() as i32;
            let top = (baseline - metrics.height as f32 - metrics.ymin as f32).round() as i32;
            let data = self.pixmap.data_mut();
            for gy in 0..metrics.height as i32 {
                for gx in 0..metrics.width as i32 {
                    let (x, y) = (left + gx, top + gy);
                    if x < 0 || y < 0 || x >= w || y >= h {
                        continue;
                    }
                    let a = coverage[(gy * metrics.width as i32 + gx) as usize] as f32 / 255.0;
                    let at = ((y * w + x) * 4) as usize;
                    for byte in &mut data[at..at + 3] {
                        *byte = (255.0 * a + *byte as f32 * (1.0 - a)).round() as u8;
                    }
                }
            }
            pen += metrics.advance_width;
            if let Some(&next) = chars.get(i + 1) {
                pen += font.horizontal_kern(c, next, px).unwrap_or(0.0);
            }
        }
    }

    /// The pixels as straight RGBA (the pictures are opaque, so the
    /// premultiplied values are the same).
    fn rgba(&self) -> Vec<u8> {
        self.pixmap.data().to_vec()
    }
}

/// A rectangle with circular corners of radius `r` (a canvas path built with
/// `arcTo`), each corner a cubic curve.
fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> SkPath {
    const K: f32 = 0.552_284_8;
    let mut p = PathBuilder::new();
    p.move_to(x + r, y);
    p.line_to(x + w - r, y);
    p.cubic_to(x + w - r + r * K, y, x + w, y + r - r * K, x + w, y + r);
    p.line_to(x + w, y + h - r);
    p.cubic_to(x + w, y + h - r + r * K, x + w - r + r * K, y + h, x + w - r, y + h);
    p.line_to(x + r, y + h);
    p.cubic_to(x + r - r * K, y + h, x, y + h - r + r * K, x, y + h - r);
    p.line_to(x, y + r);
    p.cubic_to(x, y + r - r * K, x + r - r * K, y, x + r, y);
    p.close();
    p.finish().unwrap()
}

fn icon() -> Vec<u8> {
    let mut canvas = Canvas::new(128, 128);
    canvas.background();
    // The home screen crops the icon to a circle. The selected tile sits just
    // left of the middle and the row runs off the right edge, as in the app.
    let scale = 1.3;
    canvas.mark(64.0 - (MARK_TILE.x + MARK_TILE.side / 2.0) * scale - 10.0, 64.0 - (MARK_HEIGHT / 2.0) * scale, scale, 1);
    canvas.rgba()
}

fn live_area_background() -> Vec<u8> {
    let mut canvas = Canvas::new(840, 500);
    canvas.background();
    // Large soft glows in the tiles' colors, and the row faint along the
    // bottom. The gate covers the middle and the system writes the name over
    // the left, so the picture stays dark there.
    canvas.glow(700.0, 90.0, 320.0, 0x8b5cf6, 0.28);
    canvas.glow(120.0, 440.0, 300.0, 0x06b6d4, 0.18);
    // As the canvas version drew it: the 0.35 reaches the mark's glow; each
    // tile sets its own opacity and leaves 1 for the ring.
    canvas.alpha = 0.35;
    canvas.mark(470.0, 360.0, 2.4, 3);
    canvas.alpha = 1.0;
    let shader = Shader::SolidColor(color(0x02040c, 0x55 as f32 / 255.0));
    canvas.fill_rect(0.0, 0.0, 840.0, 500.0, shader, None);
    canvas.rgba()
}

fn splash(title: &str) -> Vec<u8> {
    let mut canvas = Canvas::new(960, 544);
    canvas.background();
    canvas.mark(480.0 - MARK_WIDTH, SPLASH_MARK_TOP * 2.0, 2.0, 0);
    canvas.text_centered(title, 40.0, 480.0, SPLASH_TITLE_Y * 2.0);
    canvas.rgba()
}

fn startup() -> Vec<u8> {
    let mut canvas = Canvas::new(280, 158);
    canvas.background();
    // No text: the system's "Start" label covers the bottom of the gate. The
    // mark sits above the middle, clear of that label.
    let scale = 1.2;
    canvas.mark(140.0 - (MARK_WIDTH * scale) / 2.0, 26.0, scale, 0);
    canvas.rgba()
}

/// "0.1.0" as "0.1": the patch number only when it is not zero.
fn short_version(version: &str) -> String {
    match version.strip_suffix(".0") {
        Some(short) if short.matches('.').count() == 1 => short.to_string(),
        _ => version.to_string(),
    }
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// One line of LiveArea text in a frame, white with a shadow.
fn frame(id: &str, text: &str, size: u32, attributes: &str, bold: bool) -> String {
    let bold = if bold { " bold=\"on\"" } else { "" };
    format!(
        "  <frame id=\"{id}\">\n    <liveitem>\n      <text {attributes}>\n        <str color=\"#ffffff\" size=\"{size}\"{bold} shadow=\"on\">{}</str>\n      </text>\n    </liveitem>\n  </frame>\n",
        escape_xml(text)
    )
}

/// The LiveArea layout: the name large in frame2, "by <author>" under it in
/// frame3 (left out without an author), the version smaller in frame4.
fn template(manifest: &Manifest) -> String {
    let frames = [
        frame("frame2", &manifest.title, 50, r#"valign="bottom" align="left" text-align="left" text-valign="bottom" line-space="3" ellipsis="on""#, true),
        manifest
            .author
            .as_ref()
            .map(|a| frame("frame3", &format!("by {a}"), 22, r#"valign="top" align="left" text-align="left" text-valign="top" line-space="2" ellipsis="on""#, false))
            .unwrap_or_default(),
        frame("frame4", &format!("v{}", short_version(&manifest.version)), 18, r#"align="left" text-align="left" word-wrap="off" ellipsis="on""#, false),
    ];
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<livearea style=\"psmobile\" format-ver=\"01.00\" content-rev=\"1\">\n  <livearea-background>\n    <image>bg.png</image>\n  </livearea-background>\n  <gate>\n    <startup-image>startup.png</startup-image>\n  </gate>\n{}</livearea>\n",
        frames.concat()
    )
}

pub fn run(root: &Path) -> Result<(), String> {
    let manifest = manifest::read(root)?;
    let out = root.join("vita/package/sce_sys");
    let files: [(&str, usize, usize, Vec<u8>); 4] = [
        ("icon0.png", 128, 128, icon()),
        ("pic0.png", 960, 544, splash(&manifest.title)),
        ("livearea/contents/bg.png", 840, 500, live_area_background()),
        ("livearea/contents/startup.png", 280, 158, startup()),
    ];
    for (path, width, height, rgba) in files {
        let file = out.join(path);
        fs::create_dir_all(file.parent().unwrap()).map_err(|e| e.to_string())?;
        let png = crate::png::indexed(&rgba, width, height);
        fs::write(&file, &png).map_err(|e| format!("{path}: {e}"))?;
        println!("{path}  {width}x{height}  {} bytes", png.len());
    }
    fs::write(out.join("livearea/contents/template.xml"), template(&manifest)).map_err(|e| e.to_string())?;
    println!("livearea/contents/template.xml  {}, by {}, v{}", manifest.title, manifest.author.as_deref().unwrap_or("(no author)"), short_version(&manifest.version));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_drop_a_zero_patch() {
        assert_eq!(short_version("0.2.0"), "0.2");
        assert_eq!(short_version("0.2.1"), "0.2.1");
        assert_eq!(short_version("1.10.0"), "1.10");
    }

    /// The committed template is what the manifest gives.
    #[test]
    fn template_matches_the_committed_file() {
        let root = crate::root();
        let manifest = manifest::read(&root).unwrap();
        let committed = std::fs::read_to_string(root.join("vita/package/sce_sys/livearea/contents/template.xml")).unwrap();
        assert_eq!(template(&manifest), committed);
    }
}
