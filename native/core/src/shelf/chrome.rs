//! The shelf's frame: the top bar (`components/header.tsx`), its status
//! items (`components/status-bar.tsx`) and the footer
//! (`components/footer.tsx`).

use super::layout::{BAR_H, BAR_SIDE, PILL_ROOM, STATUS_W, TAB_GAP, TAB_MAX};
use super::texts;
use super::tween::Tween;
use crate::gfx::{Color, Gfx, Rect};
use crate::host::Status;
use crate::library::categories::Category;
use crate::library::settings::{ClockFormat, ConfirmMode, Settings};
use crate::shapes::{Icon, Symbol};
use crate::text::Role;
use crate::theme::{is_light, Theme};
use crate::W;

pub const FOOTER_H: f32 = 26.0;
/// The button pills' fixed gray (`#3d4350` in `src/icons/`).
const PILL: Color = Color::rgb(0x3d4350);
const WHITE: Color = Color::rgb(0xffffff);

/// "14:05", or "2:05 PM" on a 12-hour clock; an unknown format is 24-hour.
pub fn format_clock(hour: u8, minute: u8, clock24: Option<bool>) -> String {
    if clock24 != Some(false) {
        return format!("{hour:02}:{minute:02}");
    }
    let h12 = if hour % 12 == 0 { 12 } else { hour % 12 };
    format!("{h12}:{minute:02} {}", if hour < 12 { "AM" } else { "PM" })
}

/// Rings of the Wi-Fi symbol lit for a signal in percent: 0 (the dot alone)
/// to 3, the Vita's four levels.
pub fn wifi_level(percent: u8) -> u8 {
    (percent / 25).min(3)
}

/// A name in the top bar: its text cut to `TAB_MAX` and its width in bold,
/// so a name keeps its width when it becomes current.
struct Name {
    id: String,
    label: String,
    width: f32,
    left: f32,
}

/// The most a cut name shows at: half of it in view shows at half of this.
const CUT_OPACITY: f32 = 0.5;

/// The top bar's state: the measured names and their slide.
pub struct Header {
    names: Vec<Name>,
    /// The names' labels as last measured, to notice a change.
    measured: Vec<(String, String)>,
    scroll: Tween,
    placed: bool,
}

impl Default for Header {
    fn default() -> Self {
        Header { names: Vec::new(), measured: Vec::new(), scroll: Tween::new(0.0, 220), placed: false }
    }
}

/// What the top bar draws from.
pub struct HeaderInput<'a> {
    pub theme: &'a Theme,
    pub settings: &'a Settings,
    pub status: &'a Status,
    pub tabs: &'a [Category],
    pub current: &'a str,
    pub now: u64,
}

impl Header {
    /// The room for the names: the bar less its padding, L and R, and the
    /// status items.
    fn room(status_bar: bool) -> f32 {
        W - BAR_SIDE * 2.0 - PILL_ROOM * 2.0 - if status_bar { STATUS_W } else { 0.0 }
    }

    fn measure(&mut self, gfx: &mut Gfx, tabs: &[Category]) {
        let labels: Vec<(String, String)> = tabs.iter().map(|c| (c.id.clone(), c.label.clone())).collect();
        if labels == self.measured {
            return;
        }
        let mut x = 0.0;
        self.names = labels
            .iter()
            .map(|(id, label)| {
                let label = gfx.fit(Role::BodyBold, label, TAB_MAX);
                let width = gfx.measure(Role::BodyBold, &label);
                let name = Name { id: id.clone(), label, width, left: x };
                x += width + TAB_GAP;
                name
            })
            .collect();
        self.measured = labels;
    }

    pub fn draw(&mut self, gfx: &mut Gfx, input: &HeaderInput) {
        let theme = input.theme;
        self.measure(gfx, input.tabs);
        let room = Header::room(input.settings.status_bar);
        let index = self.names.iter().position(|n| n.id == input.current).unwrap_or(0);
        // The current name in the middle between L and R, at every press.
        let target = self.names.get(index).map_or(0.0, |n| n.left + n.width / 2.0 - room / 2.0);
        if self.placed {
            self.scroll.set(target, input.now);
        } else {
            self.scroll.jump(target);
            self.placed = true;
        }
        let scroll = self.scroll.at(input.now);

        let names_x = BAR_SIDE + PILL_ROOM;
        let many = self.names.len() > 1;
        if many {
            shoulder(gfx, true, BAR_SIDE, (BAR_H - 16.0) / 2.0);
            shoulder(gfx, false, names_x + room + 6.0, (BAR_H - 16.0) / 2.0);
        }

        gfx.clip(Some(Rect::new(names_x, 0.0, room, BAR_H)));
        let line = gfx.line_height(Role::BodyBold);
        let top = ((BAR_H - (line + 3.0 + 2.0)) / 2.0).round();
        for (i, name) in self.names.iter().enumerate() {
            let left = name.left - scroll;
            if left + name.width < 0.0 || left > room {
                continue;
            }
            // A name cut by an end of the bar shows faint, fainter the less
            // of it shows.
            let seen = (left + name.width).min(room) - left.max(0.0);
            let part = (seen / name.width.max(1.0)).clamp(0.0, 1.0);
            // Whole to within a hundredth of a unit: the slide's arithmetic
            // must not turn a name in full view faint.
            let opacity = if seen >= name.width - 0.01 { 1.0 } else { part * CUT_OPACITY };
            let active = i == index;
            let (role, color) = if active { (Role::BodyBold, theme.text) } else { (Role::Body, theme.dim) };
            let center = names_x + left + name.width / 2.0;
            gfx.text_top_centered(role, center, top, color.alpha(opacity), &name.label);
            if active {
                gfx.round_rect(Rect::new(center - 7.0, top + line + 3.0, 14.0, 2.0), 1.0, 0.0, theme.accent.alpha(opacity));
            }
        }
        gfx.clip(None);

        if input.settings.status_bar {
            status_items(gfx, theme, input.settings, input.status, W - BAR_SIDE, BAR_H / 2.0);
        }
    }

    /// Whether the names are still sliding.
    pub fn moving(&self, now: u64) -> bool {
        self.scroll.moving(now)
    }
}

/// The L or R button: the pill's shape with its letter.
fn shoulder(gfx: &mut Gfx, left: bool, x: f32, y: f32) {
    gfx.icon(if left { Icon::ShoulderL } else { Icon::ShoulderR }, x, y, PILL);
    // The letter sits toward the inner side, as in `btn-l.svg`/`btn-r.svg`.
    let (letter, cx) = if left { ("L", x + 13.8) } else { ("R", x + 10.7) };
    gfx.text_centered(Role::Label, cx, y + 11.2, WHITE, letter);
}

/// Wi-Fi, Bluetooth, the battery and the time, right-aligned at `right`,
/// centered on `cy`. A reading the host cannot give is left out; Wi-Fi
/// shows only while connected, Bluetooth only while on.
fn status_items(gfx: &mut Gfx, theme: &Theme, settings: &Settings, status: &Status, right: f32, cy: f32) {
    let clock24 = match settings.clock {
        ClockFormat::System => status.clock24,
        ClockFormat::H24 => Some(true),
        ClockFormat::H12 => Some(false),
    };
    let clock = format_clock(status.hour, status.minute, clock24);
    let width = gfx.measure(Role::BodyBold, &clock);
    let baseline = (cy - gfx.line_height(Role::BodyBold) / 2.0 + gfx.ascent(Role::BodyBold)).round();
    let mut x = right - width;
    gfx.text(Role::BodyBold, x, baseline, theme.text, &clock);
    if let Some(percent) = status.battery {
        x -= 8.0 + 27.0;
        battery(gfx, theme, settings.battery_percent, percent, status.charging, x, cy - 6.0);
    }
    if status.bluetooth == Some(true) {
        x -= 8.0 + 12.0;
        gfx.icon(Icon::Bluetooth, x, cy - 6.0, theme.text);
    }
    if let Some(wifi) = status.wifi.filter(|&w| w > 0) {
        x -= 8.0 + 12.0;
        let level = wifi_level(wifi);
        gfx.icon(Icon::Wifi { level, lit: false }, x, cy - 6.0, theme.text.with_a(0x44));
        gfx.icon(Icon::Wifi { level, lit: true }, x, cy - 6.0, theme.text);
    }
}

/// Battery body, which the charge fills from the left.
const BATTERY_W: f32 = 24.0;
const BATTERY_R: f32 = 3.0;
/// At or below this charge the battery is drawn in `LOW_BATTERY`.
const LOW_PERCENT: u8 = 15;
const LOW_BATTERY: Color = Color::rgb(0xf87171);

/// A rounded body filled from the left to the charge, the number over it,
/// a nub on the right. The charge is the text color, the accent while
/// charging, red when low.
fn battery(gfx: &mut Gfx, theme: &Theme, number: bool, percent: u8, charging: bool, x: f32, y: f32) {
    let fill_w = (BATTERY_W * percent.min(100) as f32 / 100.0).round();
    let fill = if charging {
        theme.accent
    } else if percent <= LOW_PERCENT {
        LOW_BATTERY
    } else {
        theme.text
    };
    let empty = theme.text.with_a(0x59);
    gfx.round_rect(Rect::new(x, y, BATTERY_W, 12.0), BATTERY_R, 0.0, empty);
    if fill_w >= BATTERY_R * 2.0 {
        gfx.round_rect(Rect::new(x, y, fill_w, 12.0), BATTERY_R, 0.0, fill);
        // A charge short of the right corner ends square.
        if fill_w <= BATTERY_W - BATTERY_R {
            gfx.fill(Rect::new(x + fill_w - BATTERY_R, y, BATTERY_R, 12.0), fill);
        }
    } else if fill_w > 0.0 {
        gfx.round_rect(Rect::new(x, y, fill_w, 12.0), 2.0, 0.0, fill);
    }
    if number {
        // Dark on the dark themes' light charge, light on Daylight's.
        let ink = if is_light(theme) { theme.panel } else { theme.bg_bottom };
        let baseline = y + 6.0 - gfx.line_height(Role::Tiny) / 2.0 + gfx.ascent(Role::Tiny);
        gfx.text_centered(Role::Tiny, x + BATTERY_W / 2.0, baseline.round(), ink, &percent.to_string());
    }
    gfx.round_rect(Rect::new(x + BATTERY_W + 1.0, y + 4.0, 2.0, 4.0), 1.0, 0.0, empty);
}

/// A button in a prompt.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    Face(Symbol),
    Select,
    Start,
}

impl Button {
    /// The confirm button and the other face button under a confirm setting.
    pub fn confirm(mode: ConfirmMode) -> Button {
        Button::Face(if mode == ConfirmMode::Circle { Symbol::Circle } else { Symbol::Cross })
    }

    pub fn cancel(mode: ConfirmMode) -> Button {
        Button::Face(if mode == ConfirmMode::Circle { Symbol::Cross } else { Symbol::Circle })
    }

    /// Draw at `(x, y)` (16 high); returns the width.
    pub fn draw(self, gfx: &mut Gfx, x: f32, y: f32) -> f32 {
        match self {
            Button::Face(symbol) => {
                gfx.button(symbol, x, y);
                16.0
            }
            Button::Select | Button::Start => {
                let label = if self == Button::Select { "SELECT" } else { "START" };
                let width = (gfx.measure(Role::Tiny, label) + 12.0).round();
                gfx.round_rect(Rect::new(x, y + 2.0, width, 12.0), 6.0, 0.0, PILL);
                let baseline = y + 8.0 - gfx.line_height(Role::Tiny) / 2.0 + gfx.ascent(Role::Tiny);
                gfx.text_centered(Role::Tiny, x + width / 2.0, baseline.round(), WHITE, label);
                width
            }
        }
    }
}

/// Button hints, each its button and what it does, left to right from `x`,
/// centered on `cy` (`hints` and `Prompt` in `components/icons.tsx`).
/// Returns the x after the last.
pub fn hints(gfx: &mut Gfx, theme: &Theme, items: &[(Button, &str)], mut x: f32, cy: f32) -> f32 {
    let baseline = (cy - gfx.line_height(Role::Small) / 2.0 + gfx.ascent(Role::Small)).round();
    for (i, (button, label)) in items.iter().enumerate() {
        if i > 0 {
            x += 8.0;
        }
        x += button.draw(gfx, x, cy - 8.0) + 4.0;
        x = gfx.text(Role::Small, x, baseline, theme.dim, label) + 4.0;
    }
    x
}

/// What the footer draws from.
pub struct FooterInput<'a> {
    pub theme: &'a Theme,
    pub settings: &'a Settings,
    /// Home shows its own start button, except at "Basic".
    pub own_launch: bool,
    pub search_term: &'a str,
    pub launching: Option<&'a str>,
    pub position: usize,
    pub count: usize,
}

/// Room for the search term beside the position.
const TERM_W: f32 = 96.0;

/// The button legend on the left, the position in the tab on the right; on
/// the search tab the term before the position.
pub fn footer(gfx: &mut Gfx, input: &FooterInput) {
    let theme = input.theme;
    let top = crate::H - FOOTER_H;
    let cy = top + FOOTER_H / 2.0;
    let baseline = (cy - gfx.line_height(Role::Small) / 2.0 + gfx.ascent(Role::Small)).round();
    match input.launching {
        Some(title) => {
            gfx.text(Role::SmallBold, BAR_SIDE, baseline, theme.accent, &texts::launching(title));
        }
        None => {
            let confirm = input.settings.confirm;
            let mut items: Vec<(Button, &str)> = Vec::new();
            if !input.own_launch {
                items.push((Button::confirm(confirm), texts::LAUNCH));
            }
            items.push((Button::Face(Symbol::Triangle), texts::EDIT));
            items.push((Button::Face(Symbol::Square), texts::SEARCH));
            if input.search_term.is_empty() {
                items.push((Button::Select, texts::MENU));
            } else {
                items.push((Button::cancel(confirm), texts::CLOSE));
            }
            hints(gfx, theme, &items, BAR_SIDE, cy);
        }
    }
    let position = format!("{} / {}", if input.count == 0 { 0 } else { input.position + 1 }, input.count);
    let width = gfx.measure(Role::Small, &position);
    let right = W - BAR_SIDE;
    gfx.text(Role::Small, right - width, baseline, theme.faint, &position);
    if !input.search_term.is_empty() {
        let term = format!("\"{}\"", gfx.fit(Role::SmallBold, input.search_term, TERM_W));
        gfx.text_right(Role::SmallBold, right - width - 8.0, baseline, theme.accent, &term);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_formats() {
        assert_eq!(format_clock(9, 5, Some(true)), "09:05");
        assert_eq!(format_clock(21, 47, None), "21:47");
        assert_eq!(format_clock(0, 5, Some(false)), "12:05 AM");
        assert_eq!(format_clock(11, 59, Some(false)), "11:59 AM");
        assert_eq!(format_clock(12, 0, Some(false)), "12:00 PM");
        assert_eq!(format_clock(21, 47, Some(false)), "9:47 PM");
    }

    #[test]
    fn wifi_levels() {
        let levels: Vec<u8> = [1, 24, 25, 49, 50, 74, 75, 100].into_iter().map(wifi_level).collect();
        assert_eq!(levels, [0, 0, 1, 1, 2, 2, 3, 3]);
    }
}
