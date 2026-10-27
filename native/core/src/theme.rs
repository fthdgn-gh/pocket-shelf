//! Themes (`src/themes.ts`) and the color math they use (`src/accent.ts`).

use crate::gfx::Color;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeId {
    Midnight,
    Aurora,
    Sakura,
    Ember,
    Daylight,
    Dynamic,
}

impl ThemeId {
    pub const ALL: [ThemeId; 6] =
        [ThemeId::Midnight, ThemeId::Aurora, ThemeId::Sakura, ThemeId::Ember, ThemeId::Daylight, ThemeId::Dynamic];

    /// The id saved in `settings.json`.
    pub fn key(self) -> &'static str {
        match self {
            ThemeId::Midnight => "midnight",
            ThemeId::Aurora => "aurora",
            ThemeId::Sakura => "sakura",
            ThemeId::Ember => "ember",
            ThemeId::Daylight => "daylight",
            ThemeId::Dynamic => "dynamic",
        }
    }

    pub fn from_key(key: &str) -> Option<ThemeId> {
        ThemeId::ALL.into_iter().find(|id| id.key() == key)
    }
}

pub const DEFAULT_THEME: ThemeId = ThemeId::Midnight;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Theme {
    pub id: ThemeId,
    pub name: &'static str,
    /// Screen background, top and bottom of a vertical gradient.
    pub bg_top: Color,
    pub bg_bottom: Color,
    /// Primary text, secondary text, and the weakest text.
    pub text: Color,
    pub dim: Color,
    pub faint: Color,
    /// Selected tile frame, active tab, highlighted rows and values.
    pub accent: Color,
    /// Drawers and sheets.
    pub panel: Color,
    /// Hairlines and badge outlines.
    pub line: Color,
    /// Tile background behind an icon, and keys of the keyboard.
    pub tile: Color,
    /// Dimmer over the screen while a panel is open.
    pub scrim: Color,
}

const fn theme(id: ThemeId, name: &'static str, c: [u32; 9], scrim: u32) -> Theme {
    Theme {
        id,
        name,
        bg_top: Color::rgb(c[0]),
        bg_bottom: Color::rgb(c[1]),
        text: Color::rgb(c[2]),
        dim: Color::rgb(c[3]),
        faint: Color::rgb(c[4]),
        accent: Color::rgb(c[5]),
        panel: Color::rgb(c[6]),
        line: Color::rgb(c[7]),
        tile: Color::rgb(c[8]),
        scrim: Color::rgba(scrim),
    }
}

pub const THEMES: [Theme; 6] = [
    theme(ThemeId::Midnight, "Midnight", [0x101a36, 0x02040c, 0xffffff, 0x9aa8c0, 0x55627a, 0x22d3ee, 0x0b1226, 0x2a3654, 0x1a2440], 0x00000099),
    theme(ThemeId::Aurora, "Aurora", [0x0a2e24, 0x020b08, 0xecfdf5, 0x8fb8a8, 0x4a7163, 0x34d399, 0x071c16, 0x1d4638, 0x12352b], 0x00000099),
    theme(ThemeId::Sakura, "Sakura", [0x33122b, 0x0d040b, 0xfff1f7, 0xcfa3bd, 0x7d5069, 0xf472b6, 0x210c1c, 0x4d2342, 0x3a1731], 0x00000099),
    theme(ThemeId::Ember, "Ember", [0x33180a, 0x0c0502, 0xfff7ed, 0xc2a48f, 0x7a5b45, 0xfb923c, 0x200f06, 0x4d2c18, 0x3b1e0e], 0x00000099),
    theme(ThemeId::Daylight, "Daylight", [0xf8fafc, 0xd5deea, 0x0f172a, 0x475569, 0x94a3b8, 0x4f46e5, 0xffffff, 0xcbd5e1, 0xe2e8f0], 0x0f172a80),
    theme(ThemeId::Dynamic, "Dynamic", [0x101a36, 0x02040c, 0xffffff, 0x9aa8c0, 0x55627a, 0x22d3ee, 0x0b1226, 0x2a3654, 0x1a2440], 0x00000099),
];

pub fn theme_by_id(id: ThemeId) -> Theme {
    THEMES.into_iter().find(|t| t.id == id).unwrap_or(THEMES[0])
}

/// Whether a theme's screen is light (Daylight), from the top of its background.
pub fn is_light(theme: &Theme) -> bool {
    let c = theme.bg_top;
    0.2126 * c.r as f32 + 0.7152 * c.g as f32 + 0.0722 * c.b as f32 > 128.0
}

/// Hue (0-360) and saturation (0-1) of an `0xRRGGBB` color.
pub fn hue_saturation(rgb: u32) -> (f32, f32) {
    let r = ((rgb >> 16) & 255) as f32 / 255.0;
    let g = ((rgb >> 8) & 255) as f32 / 255.0;
    let b = (rgb & 255) as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let spread = max - min;
    if spread == 0.0 {
        return (0.0, 0.0);
    }
    let lightness = (max + min) / 2.0;
    let saturation = spread / (1.0 - (2.0 * lightness - 1.0).abs());
    let sixth = if max == r {
        (((g - b) / spread) % 6.0 + 6.0) % 6.0
    } else if max == g {
        (b - r) / spread + 2.0
    } else {
        (r - g) / spread + 4.0
    };
    (sixth * 60.0, saturation.min(1.0))
}

/// The color for a hue (0-360), saturation and lightness (0-1).
pub fn hsl(hue: f32, saturation: f32, lightness: f32) -> Color {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sixth = hue / 60.0;
    let second = chroma * (1.0 - ((sixth % 2.0) - 1.0).abs());
    let (r, g, b) = if sixth < 1.0 {
        (chroma, second, 0.0)
    } else if sixth < 2.0 {
        (second, chroma, 0.0)
    } else if sixth < 3.0 {
        (0.0, chroma, second)
    } else if sixth < 4.0 {
        (0.0, second, chroma)
    } else if sixth < 5.0 {
        (second, 0.0, chroma)
    } else {
        (chroma, 0.0, second)
    };
    let lift = lightness - chroma / 2.0;
    // JS Math.round rounds .5 up; f32::round rounds it away from zero. The
    // channels are never negative, so both agree.
    let channel = |v: f32| ((v + lift) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color { r: channel(r), g: channel(g), b: channel(b), a: 255 }
}

/// The Dynamic theme's palette for a title's color (`0xRRGGBB`): a dark
/// screen in its hue and a bright accent of the same hue; a color with almost
/// no saturation gives a neutral gray palette.
pub fn dynamic_theme(rgb: u32) -> Theme {
    let (hue, saturation) = hue_saturation(rgb);
    let colored = saturation >= 0.08;
    let surface = if colored { saturation.min(0.55) } else { 0.0 };
    let hint = if colored { 0.18 } else { 0.0 };
    Theme {
        id: ThemeId::Dynamic,
        name: "Dynamic",
        bg_top: hsl(hue, surface * 0.9, 0.17),
        bg_bottom: hsl(hue, surface, 0.04),
        text: Color::rgb(0xffffff),
        dim: hsl(hue, hint, 0.72),
        faint: hsl(hue, hint, 0.42),
        accent: if colored { hsl(hue, saturation.max(0.8), 0.62) } else { Color::rgb(0xe5e7eb) },
        panel: hsl(hue, surface * 0.8, 0.09),
        line: hsl(hue, surface * 0.6, 0.25),
        tile: hsl(hue, surface * 0.7, 0.19),
        scrim: Color::rgba(0x00000099),
    }
}

/// Bit set in the host's packed accent when the icon has transparent parts.
pub const ACCENT_HAS_ALPHA: u32 = 1 << 24;

/// The box behind an icon with transparent parts: top and bottom of its
/// gradient, in the accent's hue (`boxColors` in `accent.ts`). `None` for an
/// opaque icon, which covers its tile.
pub fn box_colors(packed: Option<u32>) -> Option<(Color, Color)> {
    let packed = packed?;
    if packed & ACCENT_HAS_ALPHA == 0 {
        return None;
    }
    let (hue, saturation) = hue_saturation(packed & 0xffffff);
    let kept = saturation.min(0.6);
    Some((hsl(hue, kept, 0.36), hsl(hue, kept, 0.17)))
}

#[cfg(test)]
mod tests {
    // The cases of src/tests/accent.test.ts.
    use super::*;

    fn lightness(c: Color) -> f32 {
        let (max, min) = (c.r.max(c.g).max(c.b), c.r.min(c.g).min(c.b));
        (max as f32 + min as f32) / 510.0
    }

    #[test]
    fn reads_the_primary_hues() {
        assert_eq!(hue_saturation(0xff0000), (0.0, 1.0));
        assert_eq!(hue_saturation(0x00ff00), (120.0, 1.0));
        assert_eq!(hue_saturation(0x0000ff), (240.0, 1.0));
        let (hue, _) = hue_saturation(0xff0040);
        assert!(hue > 330.0 && hue < 360.0);
        assert_eq!(hue_saturation(0x808080), (0.0, 0.0));
        assert_eq!(hue_saturation(0x000000), (0.0, 0.0));
    }

    #[test]
    fn hsl_round_trips_known_colors() {
        assert_eq!(hsl(0.0, 1.0, 0.5).hex(), 0xff0000);
        assert_eq!(hsl(120.0, 1.0, 0.5).hex(), 0x00ff00);
        assert_eq!(hsl(240.0, 1.0, 0.5).hex(), 0x0000ff);
        assert_eq!(hsl(0.0, 0.0, 0.5).hex(), 0x808080);
        assert_eq!(hsl(200.0, 0.0, 0.0).hex(), 0x000000);
    }

    #[test]
    fn box_for_see_through_icons_only() {
        assert_eq!(box_colors(Some(0x2060e0)), None);
        assert_eq!(box_colors(None), None);
        let (top, bottom) = box_colors(Some(ACCENT_HAS_ALPHA | 0x2060e0)).unwrap();
        let (accent_hue, _) = hue_saturation(0x2060e0);
        assert!((hue_saturation(top.hex()).0 - accent_hue).abs() < 3.0);
        assert!((hue_saturation(bottom.hex()).0 - accent_hue).abs() < 3.0);
        assert!(top.b > top.r);
        assert!(bottom.b < top.b);
        assert_eq!(box_colors(Some(ACCENT_HAS_ALPHA | 0x00ff00)), Some((hsl(120.0, 0.6, 0.36), hsl(120.0, 0.6, 0.17))));
        let (top, bottom) = box_colors(Some(ACCENT_HAS_ALPHA | 0x969696)).unwrap();
        assert_eq!((top.hex(), bottom.hex()), (0x5c5c5c, 0x2b2b2b));
    }

    #[test]
    fn dynamic_screen_dark_accent_bright_same_hue() {
        let theme = dynamic_theme(0x2060e0);
        let (hue, _) = hue_saturation(0x2060e0);
        assert!(lightness(theme.bg_top) < 0.2);
        assert!(lightness(theme.bg_bottom) < lightness(theme.bg_top));
        assert!(lightness(theme.accent) > 0.55);
        for c in [theme.bg_top, theme.accent, theme.panel] {
            assert!((hue_saturation(c.hex()).0 - hue).abs() < 4.0);
        }
    }

    #[test]
    fn dynamic_gray_palette() {
        let theme = dynamic_theme(0x969696);
        for c in [theme.bg_top, theme.bg_bottom, theme.panel, theme.dim] {
            assert_eq!(hue_saturation(c.hex()).1, 0.0);
        }
        assert_eq!(theme.accent.hex(), 0xe5e7eb);
        assert!(!is_light(&theme));
        assert!(is_light(&theme_by_id(ThemeId::Daylight)));
    }
}
