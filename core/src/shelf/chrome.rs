//! The shelf's frame: the top bar, its status items and the
//! footer.

use super::layout::{BAR_H, BAR_SIDE, PILL_ROOM, TAB_GAP, TAB_MAX};
use super::tween::Tween;
use crate::gfx::{Color, Gfx, Rect};
use crate::host::Status;
use crate::i18n::fill;
use crate::locales::Messages;
use crate::library::categories::Category;
use crate::library::settings::{ClockFormat, ConfirmMode, Settings, StatusPosition};
use crate::shapes::{Icon, Symbol};
use crate::text::Role;
use crate::theme::{is_light, Theme};
use crate::W;

pub const FOOTER_H: f32 = 52.0;
/// The button pills' fixed gray (`#3d4350`, as in Pocket Shelf 0.1's icons).
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
    /// The text size they were measured at.
    measured_scale: f32,
    scroll: Tween,
    placed: bool,
}

impl Default for Header {
    fn default() -> Self {
        Header { names: Vec::new(), measured: Vec::new(), measured_scale: 0.0, scroll: Tween::new(0.0, 220), placed: false }
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

    fn measure(&mut self, gfx: &mut Gfx, tabs: &[Category]) {
        let labels: Vec<(String, String)> = tabs.iter().map(|c| (c.id.clone(), c.label.clone())).collect();
        if labels == self.measured && gfx.text_scale == self.measured_scale {
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
        self.measured_scale = gfx.text_scale;
    }

    pub fn draw(&mut self, gfx: &mut Gfx, input: &HeaderInput) {
        let theme = input.theme;
        self.measure(gfx, input.tabs);
        // The status items take the width of the ones shown (and 6 apart
        // from L or R); the names have the rest.
        let items = if input.settings.status_bar { status_parts(gfx, input.settings, input.status) } else { Vec::new() };
        let items_w = parts_width(&items);
        let status_w = if items.is_empty() { 0.0 } else { items_w + 12.0 };
        let left = input.settings.layout.position == StatusPosition::Left;
        let room = W - BAR_SIDE * 2.0 - PILL_ROOM * 2.0 - status_w;
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

        let names_x = BAR_SIDE + if left { status_w } else { 0.0 } + PILL_ROOM;
        let many = self.names.len() > 1;
        if many {
            shoulder(gfx, true, names_x - PILL_ROOM, (BAR_H - 32.0) / 2.0);
            shoulder(gfx, false, names_x + room + 12.0, (BAR_H - 32.0) / 2.0);
        }

        gfx.clip(Some(Rect::new(names_x, 0.0, room, BAR_H)));
        let line = gfx.line_height(Role::BodyBold);
        let top = ((BAR_H - (line + 6.0 + 4.0)) / 2.0).round();
        for (i, name) in self.names.iter().enumerate() {
            let left = name.left - scroll;
            if left + name.width < 0.0 || left > room {
                continue;
            }
            // A name cut by an end of the bar shows faint, fainter the less
            // of it shows.
            let seen = (left + name.width).min(room) - left.max(0.0);
            let part = (seen / name.width.max(1.0)).clamp(0.0, 1.0);
            // Whole to within two hundredths of a pixel: the slide's arithmetic
            // must not turn a name in full view faint.
            let opacity = if seen >= name.width - 0.02 { 1.0 } else { part * CUT_OPACITY };
            let active = i == index;
            let (role, color) = if active { (Role::BodyBold, theme.text) } else { (Role::Body, theme.dim) };
            let center = names_x + left + name.width / 2.0;
            gfx.text_top_centered(role, center, top, color.alpha(opacity), &name.label);
            if active {
                gfx.round_rect(Rect::new(center - 14.0, top + line + 6.0, 28.0, 4.0), 2.0, 0.0, theme.accent.alpha(opacity));
            }
        }
        gfx.clip(None);

        if !items.is_empty() {
            let x = if left { BAR_SIDE } else { W - BAR_SIDE - items_w };
            draw_parts(gfx, theme, input.settings, input.status, &items, x, BAR_H / 2.0);
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
    let (letter, cx) = if left { ("L", x + 27.6) } else { ("R", x + 21.4) };
    gfx.text_centered(Role::Label, cx, y + 22.4, WHITE, letter);
}

/// A status item shown in the top bar.
#[derive(Clone, PartialEq, Debug)]
enum Part {
    Wifi(u8),
    Bluetooth,
    Battery(u8),
    Clock(String),
}

/// The status items to show, with their widths: Wi-Fi, Bluetooth, the
/// battery and the time, each when the layout shows it and the host could
/// read it (Wi-Fi only while connected, Bluetooth only while on).
fn status_parts(gfx: &mut Gfx, settings: &Settings, status: &Status) -> Vec<(Part, f32)> {
    let layout = &settings.layout;
    let mut parts = Vec::new();
    if let Some(wifi) = status.wifi.filter(|&w| w > 0 && layout.wifi) {
        parts.push((Part::Wifi(wifi_level(wifi)), 24.0));
    }
    if status.bluetooth == Some(true) && layout.bluetooth {
        parts.push((Part::Bluetooth, 24.0));
    }
    if let Some(percent) = status.battery.filter(|_| layout.battery) {
        parts.push((Part::Battery(percent), BATTERY_W + 6.0));
    }
    if layout.clock {
        let clock24 = match settings.clock {
            ClockFormat::System => status.clock24,
            ClockFormat::H24 => Some(true),
            ClockFormat::H12 => Some(false),
        };
        let clock = format_clock(status.hour, status.minute, clock24);
        let width = gfx.measure(Role::BodyBold, &clock);
        parts.push((Part::Clock(clock), width));
    }
    parts
}

/// The items' width, 16 apart.
fn parts_width(parts: &[(Part, f32)]) -> f32 {
    parts.iter().map(|(_, w)| w).sum::<f32>() + 16.0 * parts.len().saturating_sub(1) as f32
}

/// Draw the items from `x`, centered on `cy`.
fn draw_parts(gfx: &mut Gfx, theme: &Theme, settings: &Settings, status: &Status, parts: &[(Part, f32)], mut x: f32, cy: f32) {
    for (part, width) in parts {
        match part {
            Part::Wifi(level) => {
                gfx.icon(Icon::Wifi { level: *level, lit: false }, x, cy - 12.0, theme.text.with_a(0x44));
                gfx.icon(Icon::Wifi { level: *level, lit: true }, x, cy - 12.0, theme.text);
            }
            Part::Bluetooth => gfx.icon(Icon::Bluetooth, x, cy - 12.0, theme.text),
            Part::Battery(percent) => battery(gfx, theme, settings.battery_percent, *percent, status.charging, x, cy - 12.0),
            Part::Clock(clock) => {
                let baseline = (cy - gfx.line_height(Role::BodyBold) / 2.0 + gfx.ascent(Role::BodyBold)).round();
                gfx.text(Role::BodyBold, x, baseline, theme.text, clock);
            }
        }
        x += width + 16.0;
    }
}

/// Battery body, which the charge fills from the left.
const BATTERY_W: f32 = 48.0;
const BATTERY_R: f32 = 6.0;
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
    gfx.round_rect(Rect::new(x, y, BATTERY_W, 24.0), BATTERY_R, 0.0, empty);
    if fill_w >= BATTERY_R * 2.0 {
        gfx.round_rect(Rect::new(x, y, fill_w, 24.0), BATTERY_R, 0.0, fill);
        // A charge short of the right corner ends square.
        if fill_w <= BATTERY_W - BATTERY_R {
            gfx.fill(Rect::new(x + fill_w - BATTERY_R, y, BATTERY_R, 24.0), fill);
        }
    } else if fill_w > 0.0 {
        gfx.round_rect(Rect::new(x, y, fill_w, 24.0), 4.0, 0.0, fill);
    }
    if number {
        // Dark on the dark themes' light charge, light on Daylight's.
        let ink = if is_light(theme) { theme.panel } else { theme.bg_bottom };
        let baseline = y + 12.0 - gfx.line_height(Role::Tiny) / 2.0 + gfx.ascent(Role::Tiny);
        gfx.text_centered(Role::Tiny, x + BATTERY_W / 2.0, baseline.round(), ink, &percent.to_string());
    }
    gfx.round_rect(Rect::new(x + BATTERY_W + 2.0, y + 8.0, 4.0, 8.0), 2.0, 0.0, empty);
}

/// A button in a prompt.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    Face(Symbol),
    Select,
    Start,
    /// The D-pad with its up and down arms lit (move), or left and right
    /// (change).
    DpadVertical,
    DpadHorizontal,
    /// The L and R shoulder buttons, 48 wide; both, side by side.
    ShoulderL,
    ShoulderR,
    ShoulderLR,
    /// A plus (a row that adds something).
    Plus,
}

impl Button {
    /// The confirm button and the other face button under a confirm setting.
    pub fn confirm(mode: ConfirmMode) -> Button {
        Button::Face(if mode == ConfirmMode::Circle { Symbol::Circle } else { Symbol::Cross })
    }

    pub fn cancel(mode: ConfirmMode) -> Button {
        Button::Face(if mode == ConfirmMode::Circle { Symbol::Cross } else { Symbol::Circle })
    }

    /// Draw at `(x, y)` (32 high); returns the width.
    pub fn draw(self, gfx: &mut Gfx, x: f32, y: f32) -> f32 {
        match self {
            Button::Face(symbol) => {
                gfx.button(symbol, x, y);
                32.0
            }
            Button::DpadVertical | Button::DpadHorizontal => {
                // `dpad-*.svg`: a gray cross, the two arms in use white.
                gfx.round_rect(Rect::new(x + 11.0, y + 2.0, 10.0, 28.0), 2.4, 0.0, PILL);
                gfx.round_rect(Rect::new(x + 2.0, y + 11.0, 28.0, 10.0), 2.4, 0.0, PILL);
                if self == Button::DpadVertical {
                    gfx.fill(Rect::new(x + 13.2, y + 4.0, 5.6, 6.4), WHITE);
                    gfx.fill(Rect::new(x + 13.2, y + 21.6, 5.6, 6.4), WHITE);
                } else {
                    gfx.fill(Rect::new(x + 4.0, y + 13.2, 6.4, 5.6), WHITE);
                    gfx.fill(Rect::new(x + 21.6, y + 13.2, 6.4, 5.6), WHITE);
                }
                32.0
            }
            Button::ShoulderL | Button::ShoulderR => {
                shoulder(gfx, self == Button::ShoulderL, x, y);
                48.0
            }
            Button::ShoulderLR => {
                shoulder(gfx, true, x, y);
                shoulder(gfx, false, x + 56.0, y);
                104.0
            }
            Button::Plus => {
                // `plus.svg`.
                let ink = Color::rgb(0x8b95a7);
                gfx.fill(Rect::new(x + 14.0, y + 7.0, 4.0, 18.0), ink);
                gfx.fill(Rect::new(x + 7.0, y + 14.0, 18.0, 4.0), ink);
                32.0
            }
            Button::Select | Button::Start => {
                let label = if self == Button::Select { "SELECT" } else { "START" };
                let width = (gfx.measure(Role::Tiny, label) + 24.0).round();
                gfx.round_rect(Rect::new(x, y + 4.0, width, 24.0), 12.0, 0.0, PILL);
                let baseline = y + 16.0 - gfx.line_height(Role::Tiny) / 2.0 + gfx.ascent(Role::Tiny);
                gfx.text_centered(Role::Tiny, x + width / 2.0, baseline.round(), WHITE, label);
                width
            }
        }
    }
}

/// Button hints, each its button and what it does, left to right from `x`,
/// centered on `cy`.
/// Returns the x after the last.
pub fn hints(gfx: &mut Gfx, theme: &Theme, items: &[(Button, &str)], mut x: f32, cy: f32) -> f32 {
    let baseline = (cy - gfx.line_height(Role::Small) / 2.0 + gfx.ascent(Role::Small)).round();
    for (i, (button, label)) in items.iter().enumerate() {
        if i > 0 {
            x += 16.0;
        }
        x += button.draw(gfx, x, cy - 16.0) + 8.0;
        x = gfx.text(Role::Small, x, baseline, theme.dim, label) + 8.0;
    }
    x
}

/// Width of one hint: its button, a gap and its label.
fn hint_width(gfx: &mut Gfx, button: Button, label: &str) -> f32 {
    let button_w = match button {
        Button::Face(_) | Button::DpadVertical | Button::DpadHorizontal | Button::Plus => 32.0,
        Button::ShoulderL | Button::ShoulderR => 48.0,
        Button::ShoulderLR => 104.0,
        Button::Select | Button::Start => {
            let text = if button == Button::Select { "SELECT" } else { "START" };
            (gfx.measure(Role::Tiny, text) + 24.0).round()
        }
    };
    button_w + 8.0 + gfx.measure(Role::Small, label)
}

/// Hints centered on `cx` from `top`, a hint never split: hints that do not
/// fit `max_w` go to the next line, since a translated label can be longer
/// than the English one (`Prompt` with `center`). Returns the height.
pub fn hints_centered(gfx: &mut Gfx, theme: &Theme, items: &[(Button, &str)], cx: f32, top: f32, max_w: f32) -> f32 {
    let (widths, lines) = hint_lines(gfx, items, max_w);
    for (n, line) in lines.iter().enumerate() {
        let line_w: f32 = widths[line.clone()].iter().sum::<f32>() + 24.0 * (line.len().saturating_sub(1)) as f32;
        let cy = top + n as f32 * 40.0 + 16.0;
        hints(gfx, theme, &items[line.clone()], (cx - line_w / 2.0).round(), cy);
    }
    lines.len() as f32 * 40.0 - 8.0
}

/// The height `hints_centered` will take, without drawing.
pub fn hints_height(gfx: &mut Gfx, items: &[(Button, &str)], max_w: f32) -> f32 {
    hint_lines(gfx, items, max_w).1.len() as f32 * 40.0 - 8.0
}

/// Each hint's width, and the hints on each line.
fn hint_lines(gfx: &mut Gfx, items: &[(Button, &str)], max_w: f32) -> (Vec<f32>, Vec<std::ops::Range<usize>>) {
    let widths: Vec<f32> = items.iter().map(|(b, l)| hint_width(gfx, *b, l)).collect();
    let mut lines: Vec<std::ops::Range<usize>> = Vec::new();
    let (mut start, mut width) = (0, 0.0);
    for (i, w) in widths.iter().enumerate() {
        let next = if i == start { *w } else { width + 24.0 + w };
        if i > start && next > max_w {
            lines.push(start..i);
            start = i;
            width = *w;
        } else {
            width = next;
        }
    }
    lines.push(start..items.len());
    (widths, lines)
}

/// A dim line with a button's symbol where `{button}` is
/// (`withButton`), from `x`, centered on `cy`.
pub fn text_with_button(gfx: &mut Gfx, theme: &Theme, text: &str, button: Button, x: f32, cy: f32) {
    let baseline = (cy - gfx.line_height(Role::Small) / 2.0 + gfx.ascent(Role::Small)).round();
    let (before, after) = text.split_once("{button}").unwrap_or((text, ""));
    let mut x = gfx.text(Role::Small, x, baseline, theme.dim, before.trim_end()) + 8.0;
    x += button.draw(gfx, x, cy - 16.0) + 8.0;
    gfx.text(Role::Small, x, baseline, theme.dim, after.trim_start());
}

/// What the footer draws from.
pub struct FooterInput<'a> {
    pub t: &'a Messages,
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
const TERM_W: f32 = 192.0;

/// The button legend on the left, the position in the tab on the right; on
/// the search tab the term before the position.
pub fn footer(gfx: &mut Gfx, input: &FooterInput) {
    let (theme, t) = (input.theme, input.t);
    let top = crate::H - FOOTER_H;
    let cy = top + FOOTER_H / 2.0;
    let baseline = (cy - gfx.line_height(Role::Small) / 2.0 + gfx.ascent(Role::Small)).round();
    match input.launching {
        Some(title) => {
            gfx.text(Role::SmallBold, BAR_SIDE, baseline, theme.accent, &fill(t.launching, &[("title", title)]));
        }
        None => {
            let confirm = input.settings.confirm;
            let mut items: Vec<(Button, &str)> = Vec::new();
            if !input.own_launch {
                items.push((Button::confirm(confirm), t.launch));
            }
            items.push((Button::Face(Symbol::Triangle), t.edit));
            items.push((Button::Face(Symbol::Square), t.search));
            if input.search_term.is_empty() {
                items.push((Button::Select, t.menu));
            } else {
                items.push((Button::cancel(confirm), t.close));
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
        gfx.text_right(Role::SmallBold, right - width - 16.0, baseline, theme.accent, &term);
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
