//! Drawers: a rounded card that slides in from the
//! right over a dimmed screen, with rows, a note and button hints. Every
//! panel uses it: the SELECT menu, the editor, the pickers and the rest.

use super::chrome::{hints, Button};
use super::tween::Tween;
use crate::gfx::{Gfx, Rect};
use crate::shapes::Icon;
use crate::text::Role;
use crate::theme::Theme;
use crate::{H, W};

const MARGIN: f32 = 16.0;
const PAD_X: f32 = 24.0;
const PAD_Y: f32 = 20.0;
pub const ROW_H: f32 = 52.0;

/// The arrows an active row shows: none, a right one for a row that opens
/// something, both for a value that steps.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    None,
    Right,
    Both,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Size {
    Narrow,
    Wide,
}

impl Size {
    pub fn width(self) -> f32 {
        match self {
            Size::Narrow => 464.0,
            Size::Wide => 568.0,
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
        gfx.shadow(card, 24.0, 16.0, crate::gfx::BLACK.with_a(80));
        gfx.round_rect(card, 24.0, 0.0, theme.panel);
        let mut drawer = Drawer { theme: *theme, x: x + PAD_X, w: width - 2.0 * PAD_X, y: MARGIN + PAD_Y, bottom: H - MARGIN - PAD_Y };
        let heading = title.to_uppercase();
        gfx.text_top(Role::Label, drawer.x + 16.0, drawer.y, theme.dim, &heading);
        drawer.y += gfx.line_height(Role::Label) + 4.0 + 4.0 + 4.0;
        drawer
    }

    /// A row (`OptionRow`): its label on the left, its value (text and/or
    /// a button symbol) on the right, and while active the arrows of `step`.
    pub fn row(&mut self, gfx: &mut Gfx, label: &str, value: Option<&str>, icon: Option<Button>, active: bool, step: Step) {
        let theme = self.theme;
        let rect = Rect::new(self.x, self.y, self.w, ROW_H);
        if active {
            gfx.round_rect(rect, 16.0, 0.0, theme.text.with_a(0x1c));
        }
        let role = if active { Role::BodyBold } else { Role::Body };
        let baseline = (rect.y + ROW_H / 2.0 - gfx.line_height(role) / 2.0 + gfx.ascent(role)).round();
        gfx.text(role, rect.x + 16.0, baseline, if active { theme.text } else { theme.dim }, label);
        // Right to left: arrow, icon, value, left arrow.
        let mut right = rect.right() - 16.0;
        if active && step != Step::None {
            right -= 16.0;
            gfx.icon(Icon::ArrowRight, right, rect.y + 10.0, theme.dim);
            right -= 8.0;
        }
        if let Some(icon) = icon {
            right -= 32.0;
            icon.draw(gfx, right, rect.y + 10.0);
            right -= 8.0;
        }
        if let Some(value) = value {
            let base = (rect.y + ROW_H / 2.0 - gfx.line_height(Role::BodyBold) / 2.0 + gfx.ascent(Role::BodyBold)).round();
            let width = gfx.measure(Role::BodyBold, value);
            gfx.text(Role::BodyBold, right - width, base, if active { theme.accent } else { theme.dim }, value);
            right -= width + 8.0;
        }
        if active && step == Step::Both {
            gfx.icon(Icon::ArrowLeft, right - 16.0, rect.y + 10.0, theme.dim);
        }
        self.y += ROW_H + 4.0;
    }

    /// Dim text, broken into lines, from the current row down.
    pub fn text(&mut self, gfx: &mut Gfx, text: &str) {
        let line_h = gfx.line_height(Role::Small);
        for line in gfx.wrap(Role::Small, text, self.w) {
            gfx.text_top(Role::Small, self.x, self.y, self.theme.dim, &line);
            self.y += line_h + 4.0;
        }
    }

    /// Leave `h` pixels of room from the current row down; returns its top.
    pub fn space(&mut self, h: f32) -> f32 {
        let top = self.y;
        self.y += h;
        top
    }

    /// Dim text, broken into lines, at the bottom above the hints.
    pub fn note(&mut self, gfx: &mut Gfx, text: &str, hint_lines: usize) {
        let lines = gfx.wrap(Role::Small, text, self.w);
        let line_h = gfx.line_height(Role::Small);
        let hints_h = hint_lines as f32 * (32.0 + 4.0);
        let mut y = self.bottom - hints_h - lines.len() as f32 * (line_h + 4.0);
        for line in &lines {
            gfx.text_top(Role::Small, self.x, y, self.theme.dim, line);
            y += line_h + 4.0;
        }
    }

    /// Button hints on the drawer's last line.
    pub fn hints(&mut self, gfx: &mut Gfx, items: &[(Button, &str)]) {
        let theme = self.theme;
        hints(gfx, &theme, items, self.x, self.bottom - 16.0);
    }
}
