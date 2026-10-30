//! A title's picture on a tile: its icon or box art
//! once the host has decoded it, else its initials over its tint; the ring
//! around the selected tile and its shadow.

use std::collections::{HashMap, HashSet};

use crate::gfx::{Color, Format, Gfx, Rect, TexId, BLACK};
use crate::host::{Host, Picture, PictureKey};
use crate::library::catalog::{Game, TINT_COUNT};
use crate::text::Role;
use crate::theme::box_colors;

/// Fill behind the initials of a title with no icon, one per tint: the
/// gradients of `TINT_ART` (Tailwind's 400 to 600–800 shades).
const TINT_ART: [(u32, u32); TINT_COUNT] = [
    (0xfbbf24, 0xdc2626),
    (0x34d399, 0x0f766e),
    (0x38bdf8, 0x4338ca),
    (0xc084fc, 0x5b21b6),
    (0xf472b6, 0xbe123c),
    (0x67e8f9, 0x1d4ed8),
];

const WHITE: Color = Color::rgb(0xffffff);

/// Bytes of icon and box art textures kept; past it the pictures used
/// longest ago are freed (`Art::trim`). A 128 px icon is 64 KB, so this
/// holds about 96, four times the grid's 21 around the selection. With no
/// limit, texture memory reached 16 MB in 30 s of scrolling on hardware.
pub const ART_BYTES_KEPT: usize = 6 * 1024 * 1024;

/// A decoded icon or box art.
struct Loaded {
    tex: TexId,
    /// The strongest color, `0xRRGGBB` (the Dynamic theme).
    color: Option<u32>,
    /// The box behind an icon with transparent parts: a small gradient.
    boxed: Option<TexId>,
    /// Texture bytes, the picture's and its box's.
    bytes: usize,
    /// The `Art::clock` tick it was last drawn or asked for.
    used: u64,
}

/// Up to two initials of a title (one at the list's size).
pub fn initials(title: &str, one: bool) -> String {
    let words: Vec<&str> = title.split(' ').filter(|w| !w.is_empty()).collect();
    let letters: String = if one {
        title.chars().take(1).collect()
    } else if words.len() > 1 {
        words[0].chars().take(1).chain(words[1].chars().take(1)).collect()
    } else {
        title.chars().take(2).collect()
    };
    letters.to_uppercase()
}

/// The picture that stands for a title: its box art when it has one, else
/// its own icon.
pub fn source(game: &Game) -> PictureKey {
    match &game.art {
        Some(file) => PictureKey::Art(file.clone()),
        None => PictureKey::Icon(game.id.to_string()),
    }
}

#[derive(Default)]
pub struct Art {
    loaded: HashMap<PictureKey, Option<Loaded>>,
    requested: HashSet<PictureKey>,
    tints: Option<[TexId; TINT_COUNT]>,
    /// The pictures of the titles around the selection, from the last
    /// `want`: never freed by `trim`.
    near: HashSet<PictureKey>,
    /// Counts `want` calls (one a frame), for `Loaded::used`.
    clock: u64,
    /// Sum of the loaded pictures' `bytes`.
    bytes: usize,
}

/// Bytes of a texture of `w` by `h` RGBA texels.
fn rgba_bytes(w: u32, h: u32) -> usize {
    w as usize * h as usize * 4
}

/// Bytes of an icon box's gradient (`gradient_sized(.., 32)`, 8 wide).
const BOX_BYTES: usize = 8 * 32 * 4;

impl Art {
    /// Ask for the pictures of `games` (the selected title first, then
    /// those around it) unless loaded or asked for already, and take back
    /// the requests for titles no longer among them: scrolling past a
    /// thousand titles leaves only the last few waiting.
    pub fn want<'a>(&mut self, host: &mut dyn Host, games: impl Iterator<Item = &'a Game>) {
        self.clock += 1;
        let mut near: HashSet<PictureKey> = HashSet::new();
        for game in games {
            let key = source(game);
            match self.loaded.get_mut(&key) {
                Some(Some(loaded)) => loaded.used = self.clock,
                Some(None) => {}
                None if !self.requested.contains(&key) => {
                    self.requested.insert(key.clone());
                    host.request(key.clone());
                }
                None => {}
            }
            near.insert(key);
        }
        let stale: Vec<PictureKey> = self.requested.iter().filter(|k| !near.contains(*k)).cloned().collect();
        for key in stale {
            host.cancel(&key);
            self.requested.remove(&key);
        }
        self.near = near;
    }

    /// Free the pictures used longest ago until the rest fit in
    /// `ART_BYTES_KEPT`, keeping those around the selection. A title
    /// scrolled back to asks for its picture again. Called before drawing,
    /// so no freed texture is drawn.
    pub fn trim(&mut self, gfx: &mut Gfx) {
        if self.bytes <= ART_BYTES_KEPT {
            return;
        }
        let mut old: Vec<(u64, PictureKey)> = self
            .loaded
            .iter()
            .filter_map(|(key, loaded)| loaded.as_ref().map(|l| (l.used, key)))
            .filter(|(_, key)| !self.near.contains(*key))
            .map(|(used, key)| (used, key.clone()))
            .collect();
        old.sort_by_key(|(used, _)| *used);
        for (_, key) in old {
            if self.bytes <= ART_BYTES_KEPT {
                break;
            }
            if let Some(Some(loaded)) = self.loaded.remove(&key) {
                self.release(gfx, loaded);
            }
        }
    }

    /// Free a picture's textures and take its bytes off the total.
    fn release(&mut self, gfx: &mut Gfx, loaded: Loaded) {
        gfx.textures.free(loaded.tex);
        if let Some(boxed) = loaded.boxed {
            gfx.textures.free(boxed);
        }
        self.bytes -= loaded.bytes;
    }

    /// Put a picture in the cache, freeing one it replaces.
    fn keep(&mut self, gfx: &mut Gfx, key: PictureKey, loaded: Option<Loaded>) {
        if let Some(loaded) = &loaded {
            self.bytes += loaded.bytes;
        }
        if let Some(Some(old)) = self.loaded.insert(key, loaded) {
            self.release(gfx, old);
        }
    }

    /// The host's answer for an icon or a box art; false for other pictures.
    pub fn answer(&mut self, gfx: &mut Gfx, key: &PictureKey, picture: Option<Picture>) -> bool {
        if !matches!(key, PictureKey::Icon(_) | PictureKey::Art(_)) {
            return false;
        }
        self.requested.remove(key);
        let used = self.clock;
        let loaded = picture.map(|picture| {
            let boxed = box_colors(picture.accent).map(|(top, bottom)| gfx.gradient_sized(top, bottom, 32));
            let color = picture.accent.map(|a| a & 0xffffff);
            let bytes = rgba_bytes(picture.w, picture.h) + if boxed.is_some() { BOX_BYTES } else { 0 };
            let tex = gfx.add_texture(picture.w, picture.h, Format::Rgba8, picture.rgba, true);
            Loaded { tex, color, boxed, bytes, used }
        });
        self.keep(gfx, key.clone(), loaded);
        true
    }

    /// Hand a picture already decoded (Find artwork's preview of the box
    /// art just used) to the cache under `key`, so the tile shows it without
    /// reading back the file being written.
    pub fn insert(&mut self, gfx: &mut Gfx, key: PictureKey, tex: TexId, accent: Option<u32>) {
        let boxed = box_colors(accent).map(|(top, bottom)| gfx.gradient_sized(top, bottom, 32));
        self.requested.remove(&key);
        let texture = &gfx.textures.list[tex.0 as usize];
        let bytes = rgba_bytes(texture.w, texture.h) + if boxed.is_some() { BOX_BYTES } else { 0 };
        let loaded = Loaded { tex, color: accent.map(|a| a & 0xffffff), boxed, bytes, used: self.clock };
        self.keep(gfx, key, Some(loaded));
    }

    /// Forget the art files that did not decode, after the art folder was
    /// read again: one may have been written since.
    pub fn forget_missing(&mut self) {
        self.loaded.retain(|key, loaded| loaded.is_some() || !matches!(key, PictureKey::Art(_)));
    }

    /// The texture of a loaded icon or box art, by its request.
    pub fn texture(&self, key: &PictureKey) -> Option<TexId> {
        self.loaded.get(key).and_then(|l| l.as_ref()).map(|l| l.tex)
    }

    /// A title's color for the Dynamic theme: its picture's, once loaded.
    pub fn color(&self, game: &Game) -> Option<u32> {
        self.loaded.get(&source(game)).and_then(|l| l.as_ref()).and_then(|l| l.color)
    }

    /// Draw a title's picture in `rect`. `side` is the
    /// tile's size before scaling, which sizes the initials; `one` draws one
    /// initial (the list's small tiles).
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&mut self, gfx: &mut Gfx, game: &Game, rect: Rect, side: f32, opacity: f32, icon_box: bool, one: bool) {
        let tint = WHITE.alpha(opacity);
        let clock = self.clock;
        let loaded = self.loaded.get_mut(&source(game)).and_then(|l| l.as_mut()).filter(|l| gfx.textures.ready(l.tex));
        if let Some(loaded) = loaded {
            loaded.used = clock;
            if let Some(boxed) = loaded.boxed.filter(|_| icon_box) {
                gfx.image_full(boxed, rect, tint);
            }
            gfx.image_full(loaded.tex, rect, tint);
            return;
        }
        let tints = *self.tints.get_or_insert_with(|| TINT_ART.map(|(top, bottom)| gfx.gradient_sized(Color::rgb(top), Color::rgb(bottom), 64)));
        gfx.image_full(tints[game.tint as usize % TINT_COUNT], rect, tint);
        // The current UI's text-4xl on the large tiles, text-xl on the grid,
        // text-xs on the list's: about 0.29 of the side for two letters, a
        // half for one; a step smaller, as the port's text is.
        //
        // The glyphs come from three base sizes, scaled to the tile: one
        // size per tile size filled the glyph atlas with a library of
        // icon-less RetroArch games.
        let size = if one { side * 0.5 } else { side * 0.25 };
        let base = initials_base(size);
        let k = rect.w / side * size / base as f32;
        gfx.text_scaled_centered(Role::Initials(base), rect.x + rect.w / 2.0, rect.y + rect.h / 2.0, k, tint, &initials(&game.title, one));
    }
}

/// The size the initials' glyphs are rasterized at for initials `size`
/// pixels tall: 64 for the large tiles (scaled down to 44–72), 36 for the
/// grid's, 22 for the list's.
fn initials_base(size: f32) -> u8 {
    if size >= 42.0 {
        64
    } else if size >= 28.0 {
        36
    } else {
        22
    }
}

/// The ring around the selected tile: 4 thick, 6 clear of the art, in the
/// text color, scaled with the tile (`SelectedFrame`).
pub fn selected_frame(gfx: &mut Gfx, rect: Rect, k: f32, color: Color) {
    gfx.round_rect(rect.grow(10.0 * k), 6.0 * k, 4.0 * k, color);
}

/// The soft shadow under the selected tile (`TileShadow`, `shadow-lg`).
pub fn tile_shadow(gfx: &mut Gfx, rect: Rect, opacity: f32) {
    gfx.shadow(Rect::new(rect.x, rect.y + 6.0, rect.w, rect.h), 0.0, 12.0, BLACK.with_a(90).alpha(opacity));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_take_two_words_or_two_letters() {
        assert_eq!(initials("Gravity Daze", false), "GD");
        assert_eq!(initials("VitaShell", false), "VI");
        assert_eq!(initials("VitaShell", true), "V");
        assert_eq!(initials("  Star  Harbor ", false), "SH");
    }
}

#[cfg(test)]
mod want_tests {
    use super::*;
    use crate::host::TitleInfo;
    use crate::library::catalog::load_catalog;
    use crate::library::testing::Files;

    /// Records requests and cancels.
    #[derive(Default)]
    struct Recorder {
        files: Files,
        asked: Vec<PictureKey>,
        cancelled: Vec<PictureKey>,
    }

    impl Host for Recorder {
        fn time_us(&self) -> u64 {
            0
        }
        fn read(&mut self, path: &str) -> Option<Vec<u8>> {
            self.files.read(path)
        }
        fn write(&mut self, path: &str, data: &[u8]) -> bool {
            self.files.write(path, data)
        }
        fn remove(&mut self, path: &str) -> bool {
            self.files.remove(path)
        }
        fn list(&mut self, dir: &str) -> Vec<String> {
            self.files.list(dir)
        }
        fn status(&mut self) -> crate::host::Status {
            Default::default()
        }
        fn scan(&mut self, _: bool) {}
        fn request(&mut self, key: PictureKey) {
            self.asked.push(key);
        }
        fn cancel(&mut self, key: &PictureKey) {
            self.cancelled.push(key.clone());
        }
        fn poll(&mut self, _: &mut Vec<crate::host::Event>) {}
        fn log(&mut self, _: &str) {}
        fn launch(&mut self, _: &str) -> bool {
            true
        }
        fn plugin_state(&mut self) -> crate::host::PluginState {
            crate::host::PluginState::On
        }
        fn enable_plugin(&mut self) -> bool {
            true
        }
    }

    #[test]
    fn a_handed_picture_needs_no_read_and_failed_reads_are_retried() {
        let infos = [TitleInfo { id: "PCSA00001".into(), title: "T".into(), platform: "vita".into(), adrenaline: false, console: None }];
        let mut game = load_catalog(&infos).remove(0);
        game.art = Some("PCSA00001-9.png".into());
        let (mut art, mut host, mut gfx) = (Art::default(), Recorder::default(), Gfx::new());
        // A read that came before the file landed: cached as no picture.
        art.want(&mut host, std::iter::once(&game));
        art.answer(&mut gfx, &PictureKey::Art("PCSA00001-9.png".into()), None);
        art.want(&mut host, std::iter::once(&game));
        assert_eq!(host.asked.len(), 1);
        // The folder read again: the failure is forgotten and asked again.
        art.forget_missing();
        art.want(&mut host, std::iter::once(&game));
        assert_eq!(host.asked.len(), 2);
        // A picture handed over is used as it is.
        let tex = gfx.add_texture(1, 1, Format::Rgba8, vec![0; 4], true);
        art.insert(&mut gfx, PictureKey::Art("PCSA00001-9.png".into()), tex, Some(0xff0000));
        art.want(&mut host, std::iter::once(&game));
        assert_eq!(host.asked.len(), 2);
        assert_eq!(art.texture(&PictureKey::Art("PCSA00001-9.png".into())), Some(tex));
        assert_eq!(art.color(&game), Some(0xff0000));
    }

    #[test]
    fn icons_past_the_budget_are_freed_oldest_first() {
        let count = ART_BYTES_KEPT / rgba_bytes(128, 128) + 40;
        let infos: Vec<TitleInfo> = (0..count)
            .map(|n| TitleInfo { id: format!("PCSA{n:05}"), title: format!("T {n}"), platform: "vita".into(), adrenaline: false, console: None })
            .collect();
        let games = load_catalog(&infos);
        let (mut art, mut host, mut gfx) = (Art::default(), Recorder::default(), Gfx::new());
        // Scroll through every title, three at a time, each answered.
        for start in 0..count - 2 {
            art.want(&mut host, games[start..start + 3].iter());
            for game in &games[start..start + 3] {
                let key = source(game);
                if art.texture(&key).is_none() {
                    art.answer(&mut gfx, &key, Some(Picture { w: 128, h: 128, rgba: vec![0; 128 * 128 * 4], accent: None }));
                }
            }
            art.trim(&mut gfx);
            assert!(art.bytes <= ART_BYTES_KEPT);
        }
        // The titles around the selection and the latest ones stay; the
        // first ones were freed and are asked for again.
        for game in &games[count - 3..] {
            assert!(art.texture(&source(game)).is_some());
        }
        let first = source(&games[0]);
        assert!(art.texture(&first).is_none());
        let asked = host.asked.len();
        art.want(&mut host, games[0..1].iter());
        assert_eq!(host.asked.len(), asked + 1);
        let freed = gfx.textures.list.iter().filter(|t| t.freed).count();
        assert_eq!(freed + art.loaded.values().flatten().count(), count);
    }

    #[test]
    fn icons_scrolled_past_are_taken_back() {
        let infos: Vec<TitleInfo> = (0..20)
            .map(|n| TitleInfo { id: format!("PCSA{n:05}"), title: format!("T {n}"), platform: "vita".into(), adrenaline: false, console: None })
            .collect();
        let games = load_catalog(&infos);
        let (mut art, mut host) = (Art::default(), Recorder::default());
        art.want(&mut host, games[0..3].iter());
        assert_eq!(host.asked.len(), 3);
        // Asked again for the same titles: nothing new, nothing taken back.
        art.want(&mut host, games[0..3].iter());
        assert_eq!(host.asked.len(), 3);
        assert!(host.cancelled.is_empty());
        // Scrolled on: the first two are taken back, three new ones asked.
        art.want(&mut host, games[2..6].iter());
        let mut cancelled = host.cancelled.clone();
        cancelled.sort_by_key(|k| format!("{k:?}"));
        assert_eq!(cancelled, [PictureKey::Icon("PCSA00000".into()), PictureKey::Icon("PCSA00001".into())]);
        assert_eq!(host.asked.len(), 6);
        // Scrolled back: asked again, since the request was taken back.
        art.want(&mut host, games[0..3].iter());
        assert_eq!(host.asked.len(), 8);
    }
}
