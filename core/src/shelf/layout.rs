//! Geometry shared by the views, in logical units.

use crate::library::settings::DetailLevel;

pub struct Home {
    pub tile: f32,
    pub small: f32,
    pub gap: f32,
}

/// The Home row per detail level, after the PS5 home screen: the selected
/// tile at `HOME_LEFT` at `tile`, the ones after it at `small`, `gap` apart,
/// all aligned at the top. The less text under the row, the larger the tiles.
pub fn home(detail: DetailLevel) -> Home {
    match detail {
        DetailLevel::Basic => Home { tile: 128.0, small: 84.0, gap: 12.0 },
        DetailLevel::Normal => Home { tile: 96.0, small: 64.0, gap: 12.0 },
        DetailLevel::Detailed => Home { tile: 88.0, small: 56.0, gap: 12.0 },
    }
}
pub const HOME_LEFT: f32 = 20.0;
/// Tiles drawn after the selected one; about four and a half are on screen.
pub const HOME_AFTER: i32 = 6;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct HomeSlot {
    /// Left edge (a tile scales from its top left corner).
    pub x: f32,
    pub scale: f32,
    pub opacity: f32,
}

/// Where a Home tile `offset` places after the selected one sits (negative:
/// before it, off screen).
pub fn home_slot(detail: DetailLevel, offset: i32) -> HomeSlot {
    let Home { tile, small, gap } = home(detail);
    let scale = small / tile;
    if offset == 0 {
        return HomeSlot { x: HOME_LEFT, scale: 1.0, opacity: 1.0 };
    }
    if offset < 0 {
        return HomeSlot { x: HOME_LEFT + offset as f32 * (small + gap), scale, opacity: 0.0 };
    }
    HomeSlot {
        x: HOME_LEFT + tile + gap + (offset - 1) as f32 * (small + gap),
        scale,
        opacity: if offset > HOME_AFTER - 1 { 0.0 } else { 0.8 },
    }
}

pub struct ShelfRow {
    pub tile: f32,
    pub gap: f32,
    pub row: f32,
}

/// The Carousel per detail level: a tile's side, the space between tiles,
/// and the row the tiles stand in (the selected tile is drawn at 125%, plus
/// its frame).
pub fn shelf(detail: DetailLevel) -> ShelfRow {
    match detail {
        DetailLevel::Basic => ShelfRow { tile: 124.0, gap: 26.0, row: 172.0 },
        DetailLevel::Normal => ShelfRow { tile: 112.0, gap: 24.0, row: 156.0 },
        DetailLevel::Detailed => ShelfRow { tile: 100.0, gap: 22.0, row: 140.0 },
    }
}

pub const GRID_COLUMNS: usize = 5;
pub const GRID_ROWS: usize = 2;

/// The Grid per detail level: a tile's side and the space between tiles.
/// "Detailed" adds a line under the grid, so its tiles are smaller.
pub fn grid(detail: DetailLevel) -> (f32, f32) {
    match detail {
        DetailLevel::Detailed => (64.0, 10.0),
        _ => (72.0, 10.0),
    }
}

pub const LIST_ROW_H: f32 = 30.0;
pub const LIST_GAP: f32 = 4.0;
/// Rows that fit in the list.
pub const LIST_ROWS: usize = 6;

/// The Carousel's pitch and the x that centers a tile.
pub fn carousel_layout(screen_w: f32, detail: DetailLevel) -> (f32, f32) {
    let ShelfRow { tile, gap, .. } = shelf(detail);
    (tile + gap, ((screen_w - tile) / 2.0).round())
}

/// The Cascade per detail level, after the Xbox 360 dashboard (2008): the
/// selected tile's side and the row the tiles are centered in.
pub fn cascade(detail: DetailLevel) -> (f32, f32) {
    match detail {
        DetailLevel::Basic => (144.0, 168.0),
        DetailLevel::Normal => (128.0, 152.0),
        DetailLevel::Detailed => (112.0, 136.0),
    }
}
pub const CASCADE_LEFT: f32 = 28.0;
pub const CASCADE_SHRINK: f32 = 0.8;
/// Space between the selected tile and the next one.
pub const CASCADE_GAP: f32 = 6.0;
/// Part of a tile's width behind the tile in front, from the third tile on.
pub const CASCADE_OVERLAP: f32 = 0.25;
/// Tiles shown after the selected one; the one after them waits invisible.
pub const CASCADE_DEPTH: i32 = 5;
/// Shade over a tile per step back.
const CASCADE_SHADE: [f32; 7] = [0.0, 0.3, 0.45, 0.58, 0.68, 0.76, 0.8];

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CascadeSlot {
    /// Left edge (the tile scales from its left middle).
    pub x: f32,
    /// Rise toward the horizon.
    pub y: f32,
    pub scale: f32,
    /// 0 for a tile off screen.
    pub opacity: f32,
    /// Opacity of the shade over the art.
    pub shade: f32,
    /// Paint order: the selected tile in front.
    pub z: i32,
}

/// Where a Cascade tile `offset` places after the selected one sits.
pub fn cascade_slot(detail: DetailLevel, offset: i32) -> CascadeSlot {
    let side = cascade(detail).0;
    let z = 100 - offset.abs();
    if offset < 0 {
        return CascadeSlot {
            x: CASCADE_LEFT + offset as f32 * (side + CASCADE_LEFT + CASCADE_GAP),
            y: 0.0,
            scale: 1.0,
            opacity: 0.0,
            shade: 0.0,
            z,
        };
    }
    let mut x = CASCADE_LEFT;
    let mut width = side;
    for step in 1..=offset {
        let next = width * CASCADE_SHRINK;
        x += if step == 1 { width + CASCADE_GAP } else { width - next * CASCADE_OVERLAP };
        width = next;
    }
    CascadeSlot {
        x: x.round(),
        y: -2.0 * offset as f32,
        scale: CASCADE_SHRINK.powi(offset),
        opacity: if offset > CASCADE_DEPTH { 0.0 } else { 1.0 },
        shade: CASCADE_SHADE[(offset as usize).min(CASCADE_SHADE.len() - 1)],
        z,
    }
}

// Top bar: 16 of padding, L and R (24 wide, 6 gap each) around the category
// names, the status items at the right in `STATUS_W`. Names are at most
// `TAB_MAX` wide, `TAB_GAP` apart. The status items take the width of the
// ones shown (Pocket Shelf 0.1 kept 128 for them).
pub const BAR_H: f32 = 30.0;
pub const BAR_SIDE: f32 = 16.0;
pub const PILL_ROOM: f32 = 24.0 + 6.0;
pub const TAB_MAX: f32 = 120.0;
pub const TAB_GAP: f32 = 16.0;

#[cfg(test)]
mod tests {
    use super::*;

    const LEVELS: [DetailLevel; 3] = [DetailLevel::Basic, DetailLevel::Normal, DetailLevel::Detailed];

    #[test]
    fn carousel_centers_the_first_tile() {
        for detail in LEVELS {
            let ShelfRow { tile, gap, .. } = shelf(detail);
            assert_eq!(carousel_layout(480.0, detail), (tile + gap, (480.0 - tile) / 2.0));
        }
    }

    #[test]
    fn selected_shelf_tile_clears_its_neighbors_and_fits_its_row() {
        for detail in LEVELS {
            let ShelfRow { tile, gap, row } = shelf(detail);
            let selected_half = tile * 1.25 / 2.0 + 3.0 * 1.25;
            let neighbor_half = tile * 0.9 / 2.0;
            assert!(tile + gap - selected_half - neighbor_half > 0.0);
            assert!(tile * 1.25 + 3.0 * 1.25 <= row - 8.0);
        }
    }

    #[test]
    fn home_row_keeps_the_selected_tile_at_the_left() {
        for detail in LEVELS {
            let Home { tile, small, gap } = home(detail);
            assert_eq!(home_slot(detail, 0), HomeSlot { x: HOME_LEFT, scale: 1.0, opacity: 1.0 });
            let next = home_slot(detail, 1);
            assert_eq!(next.x, HOME_LEFT + tile + gap);
            assert!(next.x - (HOME_LEFT + tile + 5.0) > 0.0);
            assert!((next.scale * tile - small).abs() < 0.01);
            assert_eq!(home_slot(detail, 2).x - next.x, small + gap);
            assert_eq!(home_slot(detail, -1).opacity, 0.0);
        }
    }

    #[test]
    fn cascade_tiles_recede() {
        for detail in LEVELS {
            let selected = cascade_slot(detail, 0);
            assert!(selected.scale == 1.0 && selected.opacity == 1.0 && selected.shade == 0.0);
            let mut previous = selected;
            for offset in 1..=CASCADE_DEPTH {
                let slot = cascade_slot(detail, offset);
                assert!(slot.x > previous.x && slot.scale < previous.scale && slot.shade > previous.shade && slot.z < previous.z);
                previous = slot;
            }
            let side = cascade(detail).0;
            assert!(cascade_slot(detail, 1).x > selected.x + side);
            assert_eq!(cascade_slot(detail, CASCADE_DEPTH + 1).opacity, 0.0);
            assert_eq!(cascade_slot(detail, -1).opacity, 0.0);
            assert!(cascade_slot(detail, -1).x + side < 0.0);
        }
    }
}
