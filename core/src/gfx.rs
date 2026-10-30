//! Draw commands and the textures they use.
//!
//! Callers work in logical units (480×272, as the current UI's layout
//! numbers are written); `Gfx` multiplies by `scale` (2 on the Vita) when it
//! records a command, so commands are in screen pixels.

use std::collections::HashMap;

use crate::text::FontState;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// `0xRRGGBB`, opaque.
    pub const fn rgb(hex: u32) -> Self {
        Color { r: (hex >> 16) as u8, g: (hex >> 8) as u8, b: hex as u8, a: 255 }
    }

    /// `0xRRGGBBAA`.
    pub const fn rgba(hex: u32) -> Self {
        Color { r: (hex >> 24) as u8, g: (hex >> 16) as u8, b: (hex >> 8) as u8, a: hex as u8 }
    }

    /// The same color with its alpha multiplied by `factor`.
    pub fn alpha(self, factor: f32) -> Self {
        Color { a: (self.a as f32 * factor.clamp(0.0, 1.0)).round() as u8, ..self }
    }

    /// The same color with alpha `a` (0-255), as the TS `alpha(color, "33")`.
    pub const fn with_a(self, a: u8) -> Self {
        Color { a, ..self }
    }

    pub fn lerp(self, other: Color, t: f32) -> Self {
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Color { r: mix(self.r, other.r), g: mix(self.g, other.g), b: mix(self.b, other.b), a: mix(self.a, other.a) }
    }

    /// `0xRRGGBB`, alpha dropped.
    pub const fn hex(self) -> u32 {
        (self.r as u32) << 16 | (self.g as u32) << 8 | self.b as u32
    }
}

pub const WHITE: Color = Color::rgb(0xffffff);
pub const BLACK: Color = Color::rgb(0x000000);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }

    pub fn grow(self, by: f32) -> Self {
        Rect::new(self.x - by, self.y - by, self.w + 2.0 * by, self.h + 2.0 * by)
    }

    pub fn lerp(self, to: Rect, t: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Rect::new(mix(self.x, to.x), mix(self.y, to.y), mix(self.w, to.w), mix(self.h, to.h))
    }

    pub fn scale(self, by: f32) -> Self {
        Rect::new(self.x * by, self.y * by, self.w * by, self.h * by)
    }

    /// Snapped to whole units, so 1:1 texture draws stay sharp.
    pub fn round(self) -> Self {
        Rect::new(self.x.round(), self.y.round(), self.w.round(), self.h.round())
    }

    pub fn right(self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(self) -> f32 {
        self.y + self.h
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TexId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    /// Four bytes a texel, straight alpha.
    Rgba8,
    /// One byte a texel, read as white with that alpha, so a tint colors it.
    A8,
}

impl Format {
    pub fn texel(self) -> usize {
        match self {
            Format::Rgba8 => 4,
            Format::A8 => 1,
        }
    }
}

pub struct Texture {
    pub w: u32,
    pub h: u32,
    pub format: Format,
    /// The texels. A backend that keeps its own copy may empty this after
    /// uploading, unless `keep` is set.
    pub pixels: Vec<u8>,
    /// Bumped on every change; a backend re-uploads when it differs.
    pub version: u32,
    /// Bilinear when true, nearest texel when false.
    pub linear: bool,
    /// The texels change later (the glyph atlas), so the CPU copy stays.
    pub keep: bool,
    /// Freed by the app: the backend releases its copy, and the slot is
    /// reused for a later texture.
    pub freed: bool,
    /// Rows changed since the backend last uploaded: `None` for all of
    /// them, an empty range for none. Lets a glyph upload a few rows of the
    /// atlas instead of all of it.
    pub dirty: Option<(u32, u32)>,
    /// Set by the backend once the current version is on the GPU. A large
    /// picture uploads over several frames and is not drawn until then.
    pub uploaded: bool,
}

#[derive(Default)]
pub struct Textures {
    pub list: Vec<Texture>,
    free_slots: Vec<u32>,
}

impl Textures {
    pub fn add(&mut self, w: u32, h: u32, format: Format, pixels: Vec<u8>, linear: bool) -> TexId {
        assert_eq!(pixels.len(), (w * h) as usize * format.texel(), "texture size");
        let texture = Texture { w, h, format, pixels, version: 1, linear, keep: false, freed: false, dirty: None, uploaded: false };
        if let Some(slot) = self.free_slots.pop() {
            let old = &mut self.list[slot as usize];
            // A new version, so a backend replaces whatever it held here.
            let version = old.version + 1;
            *old = Texture { version, ..texture };
            return TexId(slot);
        }
        self.list.push(texture);
        TexId(self.list.len() as u32 - 1)
    }

    pub fn get(&self, id: TexId) -> &Texture {
        &self.list[id.0 as usize]
    }

    /// Mutable access; marks the whole texture changed.
    pub fn edit(&mut self, id: TexId) -> &mut Texture {
        let texture = &mut self.list[id.0 as usize];
        texture.version += 1;
        texture.dirty = None;
        texture
    }

    /// Mutable access; marks rows `top..bottom` changed.
    pub fn edit_rows(&mut self, id: TexId, top: u32, bottom: u32) -> &mut Texture {
        let texture = &mut self.list[id.0 as usize];
        texture.version += 1;
        texture.dirty = match texture.dirty {
            None => None,
            Some((a, b)) if a == b => Some((top, bottom)),
            Some((a, b)) => Some((a.min(top), b.max(bottom))),
        };
        texture
    }

    /// Whether the texture is on the GPU and may be drawn.
    pub fn ready(&self, id: TexId) -> bool {
        self.list[id.0 as usize].uploaded
    }

    /// For a backend that uploads everything at once (the desktop's).
    pub fn mark_all_uploaded(&mut self) {
        for texture in &mut self.list {
            texture.uploaded = true;
            texture.dirty = Some((0, 0));
        }
    }

    /// Release a texture. The caller must not draw it again; the slot is
    /// reused from the next frame on, after the backend has let go of it.
    pub fn free(&mut self, id: TexId) {
        let texture = &mut self.list[id.0 as usize];
        if texture.freed {
            return;
        }
        texture.freed = true;
        texture.pixels = Vec::new();
        texture.version += 1;
    }

    /// Called by a backend once it has released every freed texture: their
    /// slots may be handed out again.
    pub fn recycle_freed(&mut self) {
        for (index, texture) in self.list.iter().enumerate() {
            if texture.freed && !self.free_slots.contains(&(index as u32)) {
                self.free_slots.push(index as u32);
            }
        }
    }

    /// Bytes of texels in live textures, for Diagnostics.
    pub fn live_bytes(&self) -> usize {
        self.list.iter().filter(|t| !t.freed).map(|t| (t.w * t.h) as usize * t.format.texel()).sum()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Cmd {
    Fill { rect: Rect, color: Color },
    /// Draw the `src` part of a texture (in texels) into `dst`, scaled,
    /// every texel multiplied by `tint`.
    Image { tex: TexId, src: Rect, dst: Rect, tint: Color },
}

/// What a frame did that can cost time, for the slow-frame log.
#[derive(Clone, Copy, Default, Debug)]
pub struct FrameWork {
    pub glyphs: u32,
    pub textures_added: u32,
    /// The glyph atlas started over at this frame's start.
    pub atlas_reset: bool,
}

/// Everything a frame needs: the commands and the textures they read.
pub struct Gfx {
    pub textures: Textures,
    pub cmds: Vec<Cmd>,
    /// Screen pixels per logical unit.
    pub scale: f32,
    /// The text size setting's factor on each role's size (`Gfx::style`).
    pub text_scale: f32,
    pub work: FrameWork,
    pub(crate) fonts: FontState,
    pub(crate) shapes: HashMap<u64, TexId>,
    /// The clip, in screen pixels (`Gfx::clip`).
    clip_px: Option<Rect>,
}

/// The part of `a` inside `b`; `None` when they do not overlap.
fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    let (x1, y1) = (a.right().min(b.right()), a.bottom().min(b.bottom()));
    (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
}

impl Gfx {
    pub fn new(scale: f32) -> Self {
        let mut textures = Textures::default();
        let fonts = FontState::new(&mut textures);
        Gfx { textures, cmds: Vec::with_capacity(1024), scale, text_scale: 1.0, work: FrameWork::default(), fonts, shapes: HashMap::new(), clip_px: None }
    }

    /// Start a frame: drop the previous frame's commands, keep the textures.
    pub fn begin(&mut self) {
        self.cmds.clear();
        self.clip_px = None;
        self.work = FrameWork::default();
        let resets = self.fonts.resets;
        self.fonts.begin(&mut self.textures);
        self.work.atlas_reset = self.fonts.resets != resets;
    }

    /// Times the glyph atlas started over since the start.
    pub fn atlas_resets(&self) -> u32 {
        self.fonts.resets
    }

    /// Add a texture, counted in the frame's work.
    pub fn add_texture(&mut self, w: u32, h: u32, format: Format, pixels: Vec<u8>, linear: bool) -> TexId {
        self.work.textures_added += 1;
        self.textures.add(w, h, format, pixels, linear)
    }

    pub(crate) fn raw_fill(&mut self, rect: Rect, color: Color) {
        let rect = match self.clip_px {
            Some(clip) => match intersect(rect, clip) {
                Some(cut) => cut,
                None => return,
            },
            None => rect,
        };
        if color.a > 0 && rect.w > 0.0 && rect.h > 0.0 {
            self.cmds.push(Cmd::Fill { rect, color });
        }
    }

    pub(crate) fn raw_image(&mut self, tex: TexId, src: Rect, dst: Rect, tint: Color) {
        if tint.a == 0 || dst.w <= 0.0 || dst.h <= 0.0 {
            return;
        }
        let (src, dst) = match self.clip_px {
            Some(clip) => {
                let Some(cut) = intersect(dst, clip) else { return };
                // The texels that land in the cut part: the same fractions of
                // the source as of the destination.
                let (kx, ky) = (src.w / dst.w, src.h / dst.h);
                let src = Rect::new(src.x + (cut.x - dst.x) * kx, src.y + (cut.y - dst.y) * ky, cut.w * kx, cut.h * ky);
                (src, cut)
            }
            None => (src, dst),
        };
        self.cmds.push(Cmd::Image { tex, src, dst, tint });
    }

    /// A filled rectangle.
    pub fn fill(&mut self, rect: Rect, color: Color) {
        self.raw_fill(rect.scale(self.scale), color);
    }

    /// The `src` part (texels) of a texture into `dst`.
    pub fn image(&mut self, tex: TexId, src: Rect, dst: Rect, tint: Color) {
        self.raw_image(tex, src, dst.scale(self.scale), tint);
    }

    /// The whole texture into `dst`.
    pub fn image_full(&mut self, tex: TexId, dst: Rect, tint: Color) {
        let texture = self.textures.get(tex);
        let src = Rect::new(0.0, 0.0, texture.w as f32, texture.h as f32);
        self.image(tex, src, dst, tint);
    }

    /// The texture scaled to cover `dst`, centered, cut at its sides.
    pub fn image_cover(&mut self, tex: TexId, dst: Rect, tint: Color) {
        let texture = self.textures.get(tex);
        let (tw, th) = (texture.w as f32, texture.h as f32);
        let scale = (dst.w / tw).max(dst.h / th);
        let (sw, sh) = (dst.w / scale, dst.h / scale);
        let src = Rect::new((tw - sw) / 2.0, (th - sh) / 2.0, sw, sh);
        self.image(tex, src, dst, tint);
    }

    /// Draw only inside `rect` (logical units) until the next call; `None`
    /// draws everywhere. Each rectangle is cut on the CPU, with its texture
    /// coordinates: the commands need no clipping from a backend (vita2d's
    /// goes through the stencil buffer, two full-screen draws per change,
    /// and the clipped top bar flickered on hardware).
    pub fn clip(&mut self, rect: Option<Rect>) {
        let scale = self.scale;
        self.clip_px = rect.map(|r| r.scale(scale).round());
    }
}
