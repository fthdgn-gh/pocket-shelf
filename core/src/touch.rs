//! The front touch screen: where a finger is each frame becomes a tap, a
//! drag or a flick, and a tap lands on what the last frame drew there.
//!
//! Screens record what can be tapped while they draw (`Gfx::hit`); a tap is
//! tested against the last frame's records, so the places come from the
//! drawing code itself. Most targets act as a button press the screens
//! already handle (a hint, a row's confirm, a drawer's back).

use crate::gfx::Rect;

/// What a place on the screen does when tapped.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Hit {
    /// Acts as this button (`app::buttons`): a hint, L or R, the Launch
    /// pill.
    Button(u32),
    /// A drawer's row, by its index in the drawer's list: highlighted, then
    /// confirmed.
    Row(usize),
    /// A row's left (-1) or right (1) arrow: highlighted, then stepped.
    Arrow(usize, i32),
    /// A shelf tile or list row, by its index in the tab: selected, or
    /// launched when already selected.
    Tile(usize),
    /// A tab name in the top bar, by category id.
    Tab(String),
    /// A key of the on-screen keyboard: row and column.
    Key(usize, usize),
    /// The dimmed screen around a drawer: back.
    Back,
    /// A drawer's card: takes the tap, does nothing.
    Block,
}

/// The topmost place under `(x, y)`: the one drawn last.
pub fn hit_at(hits: &[(Rect, Hit)], x: f32, y: f32) -> Option<&Hit> {
    hits.iter().rev().find(|(r, _)| x >= r.x && x < r.right() && y >= r.y && y < r.bottom()).map(|(_, h)| h)
}

/// A finger that moves less than this many pixels from where it touched
/// down is a tap when it lifts; past it, a drag.
pub const SLOP: f32 = 16.0;
/// A touch held longer than this is not a tap (a finger resting on the
/// screen).
const TAP_US: u64 = 600_000;
/// A flick keeps going at its release speed, slowing by this factor each
/// frame, until under `FLICK_STOP` pixels a frame.
const FLICK_DECAY: f32 = 0.9;
const FLICK_STOP: f32 = 2.0;
/// Frames of movement the release speed is averaged over.
const SPEED_FRAMES: usize = 3;

/// One frame's touch, as the shelf uses it.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Gesture {
    /// A tap, at the point it touched down.
    pub tap: Option<(f32, f32)>,
    /// Movement this frame, in pixels, while dragging or flicking.
    pub moved: (f32, f32),
    /// Where the drag touched down: what it scrolls is chosen there.
    pub from: Option<(f32, f32)>,
    /// A finger is on the screen (a flick has stopped).
    pub down: bool,
}

/// Turns where the finger is each frame into gestures.
#[derive(Default)]
pub struct Touch {
    /// Where the finger touched down, and when.
    start: Option<((f32, f32), u64)>,
    last: (f32, f32),
    dragging: bool,
    /// The last frames' movement, newest last.
    recent: Vec<(f32, f32)>,
    /// A flick's speed and where its drag began.
    flick: Option<((f32, f32), (f32, f32))>,
}

impl Touch {
    /// The gesture for this frame, from the finger's position (`None` when
    /// no finger is on the front screen) at `now` µs.
    pub fn update(&mut self, finger: Option<(f32, f32)>, now: u64) -> Gesture {
        match (finger, self.start) {
            (Some(at), None) => {
                // Touching down stops a flick.
                self.start = Some((at, now));
                self.last = at;
                self.dragging = false;
                self.recent.clear();
                self.flick = None;
                Gesture { down: true, ..Gesture::default() }
            }
            (Some(at), Some((from, _))) => {
                let moved = (at.0 - self.last.0, at.1 - self.last.1);
                self.last = at;
                if !self.dragging && ((at.0 - from.0).powi(2) + (at.1 - from.1).powi(2)).sqrt() >= SLOP {
                    self.dragging = true;
                    // The slop's distance moves too, so the row does not
                    // jump when the drag starts.
                    let whole = (at.0 - from.0, at.1 - from.1);
                    self.remember(whole);
                    return Gesture { moved: whole, from: Some(from), down: true, ..Gesture::default() };
                }
                if self.dragging {
                    self.remember(moved);
                    return Gesture { moved, from: Some(from), down: true, ..Gesture::default() };
                }
                Gesture { down: true, ..Gesture::default() }
            }
            (None, Some((from, at))) => {
                self.start = None;
                if !self.dragging {
                    let tap = (now.saturating_sub(at) <= TAP_US).then_some(from);
                    return Gesture { tap, ..Gesture::default() };
                }
                let n = self.recent.len().max(1) as f32;
                let speed = self.recent.iter().fold((0.0, 0.0), |s, m| (s.0 + m.0, s.1 + m.1));
                let speed = (speed.0 / n, speed.1 / n);
                if speed.0.abs().max(speed.1.abs()) >= FLICK_STOP {
                    self.flick = Some((speed, from));
                }
                self.step_flick()
            }
            (None, None) => self.step_flick(),
        }
    }

    fn remember(&mut self, moved: (f32, f32)) {
        self.recent.push(moved);
        if self.recent.len() > SPEED_FRAMES {
            self.recent.remove(0);
        }
    }

    /// A flick's movement this frame, slowing down.
    fn step_flick(&mut self) -> Gesture {
        let Some((speed, from)) = self.flick else { return Gesture::default() };
        let next = (speed.0 * FLICK_DECAY, speed.1 * FLICK_DECAY);
        self.flick = (next.0.abs().max(next.1.abs()) >= FLICK_STOP).then_some((next, from));
        Gesture { moved: speed, from: Some(from), ..Gesture::default() }
    }

    /// Stop a flick (the view it scrolls changed).
    pub fn stop(&mut self) {
        self.flick = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(touch: &mut Touch, frames: &[Option<(f32, f32)>]) -> Vec<Gesture> {
        frames.iter().enumerate().map(|(i, f)| touch.update(*f, i as u64 * 16_667)).collect()
    }

    #[test]
    fn a_short_touch_that_stays_put_is_a_tap() {
        let mut touch = Touch::default();
        let g = run(&mut touch, &[Some((100.0, 50.0)), Some((105.0, 52.0)), None]);
        assert_eq!(g[2].tap, Some((100.0, 50.0)));
        assert_eq!(g[1].moved, (0.0, 0.0));
    }

    #[test]
    fn a_resting_finger_is_not_a_tap() {
        let mut touch = Touch::default();
        let mut frames = vec![Some((100.0, 50.0)); 40];
        frames.push(None);
        assert_eq!(run(&mut touch, &frames).last().unwrap().tap, None);
    }

    #[test]
    fn a_drag_moves_from_its_start_and_flicks_on_release() {
        let mut touch = Touch::default();
        let g = run(&mut touch, &[Some((300.0, 100.0)), Some((290.0, 100.0)), Some((270.0, 100.0)), Some((240.0, 100.0)), None, None]);
        // Under the slop: nothing yet; past it, the whole distance at once.
        assert_eq!(g[1].moved, (0.0, 0.0));
        assert_eq!(g[2].moved, (-30.0, 0.0));
        assert_eq!(g[2].from, Some((300.0, 100.0)));
        assert_eq!(g[3].moved, (-30.0, 0.0));
        // Released: no tap, the flick goes on and slows down.
        assert_eq!(g[4].tap, None);
        assert_eq!(g[4].moved, (-30.0, 0.0));
        assert!(g[5].moved.0 > -30.0 && g[5].moved.0 < 0.0);
        let mut left = 0;
        while touch.update(None, 0).moved != (0.0, 0.0) {
            left += 1;
            assert!(left < 100, "a flick stops");
        }
        // Touching down stops a flick at once.
        let g = run(&mut touch, &[Some((10.0, 10.0)), Some((10.0, 30.0)), Some((10.0, 70.0)), None]);
        assert!(g[3].moved.1 > 0.0);
        assert_eq!(touch.update(Some((5.0, 5.0)), 0).moved, (0.0, 0.0));
    }

    #[test]
    fn the_last_place_drawn_is_on_top() {
        let hits = vec![(Rect::new(0.0, 0.0, 960.0, 544.0), Hit::Back), (Rect::new(400.0, 20.0, 500.0, 500.0), Hit::Block), (Rect::new(420.0, 80.0, 460.0, 52.0), Hit::Row(0))];
        assert_eq!(hit_at(&hits, 10.0, 10.0), Some(&Hit::Back));
        assert_eq!(hit_at(&hits, 450.0, 300.0), Some(&Hit::Block));
        assert_eq!(hit_at(&hits, 450.0, 100.0), Some(&Hit::Row(0)));
        assert_eq!(hit_at(&[], 1.0, 1.0), None);
    }
}
