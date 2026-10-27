//! Moving the selection (`src/navigation.ts`, the shelf part of
//! `src/input.ts`).

use super::layout::{CASCADE_DEPTH, GRID_COLUMNS, GRID_ROWS, HOME_AFTER, LIST_ROWS};
use crate::library::settings::ViewMode;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// A held direction moves once, waits 0.4 s, then repeats 10 times a second
/// (in 60 Hz frames).
pub const REPEAT_DELAY: u32 = 24;
pub const REPEAT_EVERY: u32 = 6;

/// Items L and R jump over in each view.
pub fn page_size(view: ViewMode) -> usize {
    match view {
        ViewMode::Grid => GRID_COLUMNS * GRID_ROWS,
        ViewMode::List => LIST_ROWS,
        _ => 1,
    }
}

/// How many items around the selection get their icons loaded.
pub fn icon_radius(view: ViewMode) -> usize {
    match view {
        ViewMode::Grid => GRID_COLUMNS * 2,
        ViewMode::List => LIST_ROWS + 2,
        ViewMode::Cascade => CASCADE_DEPTH as usize + 1,
        ViewMode::Home => HOME_AFTER as usize,
        ViewMode::Carousel => 3,
    }
}

/// The index a D-pad press selects, or `None` when it goes nowhere (an end
/// of the list, or an axis the view does not use). With `wrap`, a press past
/// an end continues at the other one; in the grid, up from the first row
/// lands in the same column of the last row.
pub fn move_selection(view: ViewMode, direction: Direction, index: usize, count: usize, wrap: bool) -> Option<usize> {
    let back = matches!(direction, Direction::Left | Direction::Up);
    let horizontal = matches!(direction, Direction::Left | Direction::Right);
    let (index, count, columns) = (index as i64, count as i64, GRID_COLUMNS as i64);
    let step = if back { -1 } else { 1 };
    let next = match view {
        ViewMode::Carousel | ViewMode::Cascade | ViewMode::Home => horizontal.then_some(index + step),
        ViewMode::List => (!horizontal).then_some(index + step),
        ViewMode::Grid if horizontal => Some(index + step),
        ViewMode::Grid => {
            let next = index + step * columns;
            // A partial last row still holds a destination below a longer
            // column.
            Some(if next >= count && index / columns < (count - 1) / columns { count - 1 } else { next })
        }
    }?;
    if (0..count).contains(&next) {
        return Some(next as usize);
    }
    if !wrap || count < 2 {
        return None;
    }
    if view != ViewMode::Grid || horizontal {
        return Some(if next < 0 { count - 1 } else { 0 } as usize);
    }
    // Grid, up or down: the same column at the other end; a last row too
    // short for it gives its last item.
    let column = index % columns;
    let last_row = (count - 1) / columns * columns;
    if last_row == 0 {
        return None;
    }
    Some(if next < 0 { (last_row + column).min(count - 1) } else { column } as usize)
}

/// Items one repeat of a held direction moves on the shelf: one, then five
/// once held 1.2 s, then twenty once held 3 s (frames since the press).
pub const HOLD_STAGES: [(u32, usize); 3] = [(0, 1), (72, 5), (180, 20)];

pub fn hold_steps(held_frames: u32) -> usize {
    HOLD_STAGES.iter().filter(|(after, _)| held_frames >= *after).map(|(_, steps)| *steps).last().unwrap_or(1)
}

/// `steps` moves without wrapping, stopping at the last item reached; `None`
/// when not even one move is possible.
pub fn move_selection_by(view: ViewMode, direction: Direction, index: usize, count: usize, steps: usize) -> Option<usize> {
    let mut current = None;
    for _ in 0..steps {
        match move_selection(view, direction, current.unwrap_or(index), count, false) {
            Some(next) => current = Some(next),
            None => break,
        }
    }
    current
}

/// The D-pad as the shelf reads it: a press moves and wraps, a held
/// direction repeats, further the longer it is held, and stops at the ends.
#[derive(Default)]
pub struct HeldDirection {
    held: Option<Direction>,
    frames: u32,
}

/// What the D-pad asks for this frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    /// A new press: one move, wrapping.
    Press(Direction),
    /// A repeat of a held direction: this many moves, not wrapping.
    Repeat(Direction, usize),
}

impl HeldDirection {
    /// Start a held direction's count over without a new press: after a tab
    /// change, a direction still held waits the first delay again and moves
    /// one at a time before it speeds up.
    pub fn restart(&mut self) {
        self.frames = 0;
    }

    /// The first direction held this frame, in the order left, right, up,
    /// down (`DIRECTIONS` in `input.ts`).
    pub fn update(&mut self, held: Option<Direction>) -> Option<Step> {
        let Some(direction) = held else {
            self.held = None;
            return None;
        };
        if self.held != Some(direction) {
            self.held = Some(direction);
            self.frames = 0;
            return Some(Step::Press(direction));
        }
        self.frames += 1;
        (self.frames >= REPEAT_DELAY && (self.frames - REPEAT_DELAY) % REPEAT_EVERY == 0)
            .then(|| Step::Repeat(direction, hold_steps(self.frames)))
    }
}

#[cfg(test)]
mod tests {
    use super::Direction::*;
    use super::*;
    use ViewMode::*;

    const GRID_COUNT: usize = GRID_COLUMNS * 2 + 2;

    #[test]
    fn carousel_and_list_use_one_axis() {
        assert_eq!(move_selection(Carousel, Right, 0, 3, false), Some(1));
        assert_eq!(move_selection(Carousel, Left, 1, 3, false), Some(0));
        assert_eq!(move_selection(Carousel, Up, 1, 3, false), None);
        assert_eq!(move_selection(Carousel, Down, 1, 3, false), None);
        assert_eq!(move_selection(List, Down, 0, 3, false), Some(1));
        assert_eq!(move_selection(List, Up, 1, 3, false), Some(0));
        assert_eq!(move_selection(List, Left, 1, 3, false), None);
        assert_eq!(move_selection(List, Right, 1, 3, false), None);
    }

    #[test]
    fn presses_at_either_end_go_nowhere() {
        assert_eq!(move_selection(Carousel, Left, 0, 3, false), None);
        assert_eq!(move_selection(Carousel, Right, 2, 3, false), None);
        assert_eq!(move_selection(List, Up, 0, 3, false), None);
        assert_eq!(move_selection(List, Down, 2, 3, false), None);
        assert_eq!(move_selection(Grid, Left, 0, 10, false), None);
        assert_eq!(move_selection(Grid, Up, 2, 10, false), None);
    }

    #[test]
    fn grid_moves_by_item_and_row() {
        assert_eq!(move_selection(Grid, Right, GRID_COLUMNS - 1, GRID_COUNT, false), Some(GRID_COLUMNS));
        assert_eq!(move_selection(Grid, Left, GRID_COLUMNS, GRID_COUNT, false), Some(GRID_COLUMNS - 1));
        assert_eq!(move_selection(Grid, Down, 1, GRID_COUNT, false), Some(1 + GRID_COLUMNS));
        assert_eq!(move_selection(Grid, Up, 1 + GRID_COLUMNS, GRID_COUNT, false), Some(1));
        assert_eq!(move_selection(Grid, Down, GRID_COLUMNS * 2 - 1, GRID_COUNT, false), Some(GRID_COUNT - 1));
        assert_eq!(move_selection(Grid, Down, GRID_COLUMNS * 2, GRID_COUNT, false), None);
    }

    #[test]
    fn an_empty_list_goes_nowhere() {
        for view in [Carousel, Grid, List] {
            for direction in [Left, Right, Up, Down] {
                assert_eq!(move_selection(view, direction, 0, 0, false), None);
                assert_eq!(move_selection(view, direction, 0, 0, true), None);
            }
        }
    }

    #[test]
    fn wrap_continues_at_the_other_end() {
        assert_eq!(move_selection(Carousel, Right, 2, 3, true), Some(0));
        assert_eq!(move_selection(Carousel, Left, 0, 3, true), Some(2));
        assert_eq!(move_selection(List, Down, 2, 3, true), Some(0));
        assert_eq!(move_selection(List, Up, 0, 3, true), Some(2));
        assert_eq!(move_selection(Grid, Right, GRID_COUNT - 1, GRID_COUNT, true), Some(0));
        assert_eq!(move_selection(Grid, Left, 0, GRID_COUNT, true), Some(GRID_COUNT - 1));
        assert_eq!(move_selection(Carousel, Right, 0, 3, true), Some(1));
        assert_eq!(move_selection(Carousel, Up, 0, 3, true), None);
        assert_eq!(move_selection(List, Left, 0, 3, true), None);
    }

    #[test]
    fn grid_wrap_keeps_the_column() {
        assert_eq!(move_selection(Grid, Up, 1, GRID_COUNT, true), Some(GRID_COLUMNS * 2 + 1));
        assert_eq!(move_selection(Grid, Down, GRID_COLUMNS * 2 + 1, GRID_COUNT, true), Some(1));
        assert_eq!(move_selection(Grid, Up, 3, GRID_COUNT, true), Some(GRID_COUNT - 1));
        assert_eq!(move_selection(Grid, Up, 1, 3, true), None);
        assert_eq!(move_selection(Grid, Down, 1, 3, true), None);
    }

    #[test]
    fn a_list_of_one_has_nowhere_to_go() {
        assert_eq!(move_selection(Carousel, Right, 0, 1, true), None);
        assert_eq!(move_selection(List, Up, 0, 1, true), None);
    }

    #[test]
    fn holding_moves_further_the_longer_it_is_held() {
        assert_eq!(hold_steps(24), 1);
        assert_eq!(hold_steps(71), 1);
        assert_eq!(hold_steps(72), 5);
        assert_eq!(hold_steps(179), 5);
        assert_eq!(hold_steps(180), 20);
        assert_eq!(hold_steps(10_000), 20);
    }

    #[test]
    fn move_by_stops_at_an_end() {
        assert_eq!(move_selection_by(Carousel, Right, 0, 100, 5), Some(5));
        assert_eq!(move_selection_by(Carousel, Right, 97, 100, 20), Some(99));
        assert_eq!(move_selection_by(Carousel, Right, 99, 100, 20), None);
        assert_eq!(move_selection_by(List, Up, 3, 100, 20), Some(0));
        assert_eq!(move_selection_by(Grid, Down, 2, 100, 5), Some(2 + 5 * GRID_COLUMNS));
        assert_eq!(move_selection_by(Home, Left, 10, 100, 5), Some(5));
    }

    #[test]
    fn paging_and_icon_loading() {
        assert_eq!(page_size(Carousel), 1);
        assert_eq!(page_size(Grid), GRID_COLUMNS * GRID_ROWS);
        assert_eq!(page_size(List), LIST_ROWS);
        assert!(icon_radius(Grid) >= GRID_COLUMNS * GRID_ROWS);
        assert!(icon_radius(List) >= LIST_ROWS);
        assert!(icon_radius(Cascade) >= CASCADE_DEPTH as usize);
    }

    #[test]
    fn cascade_moves_like_the_shelf() {
        assert_eq!(move_selection(Cascade, Right, 0, 3, false), Some(1));
        assert_eq!(move_selection(Cascade, Down, 0, 3, false), None);
        assert_eq!(move_selection(Cascade, Right, 2, 3, true), Some(0));
    }

    #[test]
    fn restart_drops_back_to_single_steps() {
        let mut held = HeldDirection::default();
        held.update(Some(Down));
        let mut last = None;
        for _ in 0..200 {
            last = held.update(Some(Down)).or(last);
        }
        assert_eq!(last, Some(Step::Repeat(Down, 20)));
        held.restart();
        // No press of its own; the first repeat after the delay is one step.
        let steps: Vec<Step> = (0..REPEAT_DELAY).filter_map(|_| held.update(Some(Down))).collect();
        assert_eq!(steps, [Step::Repeat(Down, 1)]);
    }

    #[test]
    fn held_direction_presses_then_repeats() {
        let mut held = HeldDirection::default();
        assert_eq!(held.update(Some(Right)), Some(Step::Press(Right)));
        let steps: Vec<Option<Step>> = (0..REPEAT_DELAY + REPEAT_EVERY).map(|_| held.update(Some(Right))).collect();
        assert_eq!(steps.iter().flatten().count(), 2);
        assert_eq!(steps[REPEAT_DELAY as usize - 1], Some(Step::Repeat(Right, 1)));
        assert_eq!(held.update(None), None);
        assert_eq!(held.update(Some(Right)), Some(Step::Press(Right)));
    }
}
