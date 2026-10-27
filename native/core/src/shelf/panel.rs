//! Drawers (`components/panel.tsx`): a rounded card that slides in from the
//! right over a dimmed screen, with rows, a note and button hints. The
//! Adrenaline drawer uses it now; the SELECT menu, the editor and the
//! pickers (phase 4) will.

use super::chrome::{hints, Button};
use super::tween::Tween;
use crate::gfx::{Gfx, Rect};
use crate::shapes::Icon;
use crate::text::Role;
use crate::theme::Theme;
use crate::{H, W};

const MARGIN: f32 = 8.0;
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 10.0;
pub const ROW_H: f32 = 26.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Size {
    Narrow,
    Wide,
}

impl Size {
    pub fn width(self) -> f32 {
        match self {
            Size::Narrow => 232.0,
            Size::Wide => 284.0,
        }
    }
}

/// A drawer being drawn: where its content goes, top to bottom.
pub struct Drawer {
    pub theme: Theme,
    /// Left edge and width of the content.
    pub x: f32,
    pub w: f32,
    /// Where the next row goes, and the bottom of the content.
    pub y: f32,
    pub bottom: f32,
}

/// The drawer's slide: 180 ms from off screen.
pub fn slide() -> Tween {
    Tween::new(0.0, 180)
}

impl Drawer {
    /// The scrim, the card at its place along `slide` (0 off screen, 1 in),
    /// and the heading in capitals.
    pub fn open(gfx: &mut Gfx, theme: &Theme, title: &str, size: Size, slide: f32) -> Drawer {
        gfx.fill(Rect::new(0.0, 0.0, W, H), theme.scrim.alpha(slide.min(1.0)));
        let width = size.width();
        let x = W - MARGIN - width + (1.0 - slide) * (width + MARGIN);
        let card = Rect::new(x, MARGIN, width, H - 2.0 * MARGIN);
        gfx.shadow(card, 12.0, 8.0, crate::gfx::BLACK.with_a(80));
        gfx.round_rect(card, 12.0, 0.0, theme.panel);
        let mut drawer = Drawer { theme: *theme, x: x + PAD_X, w: width - 2.0 * PAD_X, y: MARGIN + PAD_Y, bottom: H - MARGIN - PAD_Y };
        let heading = title.to_uppercase();
        gfx.text_top(Role::Label, drawer.x + 8.0, drawer.y, theme.dim, &heading);
        drawer.y += gfx.line_height(Role::Label) + 2.0 + 2.0 + 2.0;
        drawer
    }

    /// A row: its label, and with `arrow`, the right arrow of a row that
    /// opens something.
    pub fn row(&mut self, gfx: &mut Gfx, label: &str, value: Option<&str>, active: bool, arrow: bool) {
        let theme = self.theme;
        let rect = Rect::new(self.x, self.y, self.w, ROW_H);
        if active {
            gfx.round_rect(rect, 8.0, 0.0, theme.text.with_a(0x1c));
        }
        let role = if active { Role::BodyBold } else { Role::Body };
        let baseline = (rect.y + ROW_H / 2.0 - gfx.line_height(role) / 2.0 + gfx.ascent(role)).round();
        gfx.text(role, rect.x + 8.0, baseline, if active { theme.text } else { theme.dim }, label);
        let mut right = rect.right() - 8.0;
        if arrow && active {
            right -= 8.0;
            gfx.icon(Icon::ArrowRight, right, rect.y + 5.0, theme.dim);
            right -= 4.0;
        }
        if let Some(value) = value {
            let base = (rect.y + ROW_H / 2.0 - gfx.line_height(Role::BodyBold) / 2.0 + gfx.ascent(Role::BodyBold)).round();
            gfx.text_right(Role::BodyBold, right, base, if active { theme.accent } else { theme.dim }, value);
        }
        self.y += ROW_H + 2.0;
    }

    /// Dim text, broken into lines, at the bottom above the hints.
    pub fn note(&mut self, gfx: &mut Gfx, text: &str, hint_lines: usize) {
        let lines = gfx.wrap(Role::Small, text, self.w);
        let line_h = gfx.line_height(Role::Small);
        let hints_h = hint_lines as f32 * (16.0 + 2.0);
        let mut y = self.bottom - hints_h - lines.len() as f32 * (line_h + 2.0);
        for line in &lines {
            gfx.text_top(Role::Small, self.x, y, self.theme.dim, line);
            y += line_h + 2.0;
        }
    }

    /// Button hints on the drawer's last line.
    pub fn hints(&mut self, gfx: &mut Gfx, items: &[(Button, &str)]) {
        let theme = self.theme;
        hints(gfx, &theme, items, self.x, self.bottom - 8.0);
    }
}
