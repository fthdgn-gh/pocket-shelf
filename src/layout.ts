// Geometry shared by the views. The class literals in views/ and components/
// spell the same sizes (for example `w-[84]`), so change both together.

import type { DetailLevel } from "./types.ts";

/**
 * Home row geometry per detail level, after the PS5 home screen: the selected
 * tile stands at `HOME_LEFT` at `tile` and the ones after it follow at
 * `small`, `gap` apart, all aligned at the top. The less text sits under the
 * row, the larger the tiles.
 */
export const HOME: Record<DetailLevel, { tile: number; small: number; gap: number }> = {
  basic: { tile: 128, small: 84, gap: 12 },
  normal: { tile: 96, small: 64, gap: 12 },
  detailed: { tile: 88, small: 56, gap: 12 },
};
export const HOME_LEFT = 20;
/** Tiles mounted after the selected one; about four and a half are on screen. */
export const HOME_AFTER = 6;

export interface HomeSlot {
  /** Left edge (a tile scales from its top left corner). */
  x: number;
  scale: number;
  opacity: number;
}

/** Where a home tile `offset` places after the selected one sits (negative: before it, off screen). */
export function homeSlot(detail: DetailLevel, offset: number): HomeSlot {
  const { tile, small, gap } = HOME[detail];
  const scale = small / tile;
  if (offset === 0) return { x: HOME_LEFT, scale: 1, opacity: 1 };
  if (offset < 0) return { x: HOME_LEFT + offset * (small + gap), scale, opacity: 0 };
  return {
    x: HOME_LEFT + tile + gap + (offset - 1) * (small + gap),
    scale,
    opacity: offset > HOME_AFTER - 1 ? 0 : 0.8,
  };
}

/**
 * Shelf (carousel) geometry per detail level. The less text sits under the
 * shelf, the larger the tiles: `tile` is a tile's side, `gap` the space
 * between tiles, and `row` the height of the row the tiles stand in (the
 * selected tile is drawn at 125%, plus its frame).
 */
export const SHELF: Record<DetailLevel, { tile: number; gap: number; row: number }> = {
  basic: { tile: 124, gap: 26, row: 172 },
  normal: { tile: 112, gap: 24, row: 156 },
  detailed: { tile: 100, gap: 22, row: 140 },
};

export const GRID_COLUMNS = 5;
export const GRID_ROWS = 2;
/**
 * Grid geometry per detail level: `tile` is a tile's side and `gap` the space
 * between tiles. "Detailed" adds a second line under the grid, so its tiles
 * are smaller to keep the footer on screen under the status bar.
 */
export const GRID: Record<DetailLevel, { tile: number; gap: number }> = {
  basic: { tile: 72, gap: 10 },
  normal: { tile: 72, gap: 10 },
  detailed: { tile: 64, gap: 10 },
};

export const LIST_ROW_H = 30;
export const LIST_GAP = 4;
/** Rows that fit in the list viewport. */
export const LIST_ROWS = 6;

/**
 * List rows that fit. The status items share the top bar with the
 * categories, so the status bar setting no longer takes a row.
 */
export function listRows(_statusBar: boolean): number {
  return LIST_ROWS;
}

export interface CarouselLayout {
  cardPitch: number;
  /** translateX that centers the first card in a viewport `screenW` wide. */
  centerX: number;
}

export function carouselLayout(screenW: number, detail: DetailLevel): CarouselLayout {
  const { tile, gap } = SHELF[detail];
  return {
    cardPitch: tile + gap,
    centerX: Math.round((screenW - tile) / 2),
  };
}

/**
 * Cascade geometry, after the Xbox 360 dashboard (2008): the selected tile
 * stands at the left at full size and the next ones recede to the right, each
 * `CASCADE_SHRINK` of the one before it, partly behind it, and darker. Tiles
 * before the selection slide off the left edge. `tile` is the selected tile's
 * side and `row` the height of the row the tiles are centered in.
 */
export const CASCADE: Record<DetailLevel, { tile: number; row: number }> = {
  basic: { tile: 144, row: 168 },
  normal: { tile: 128, row: 152 },
  detailed: { tile: 112, row: 136 },
};
export const CASCADE_LEFT = 28;
export const CASCADE_SHRINK = 0.8;
/** Space between the selected tile and the next one. */
export const CASCADE_GAP = 6;
/** Part of a tile's width hidden behind the tile in front of it, from the third tile on. */
export const CASCADE_OVERLAP = 0.25;
/** Tiles shown after the selected one; the one after them waits invisible. */
export const CASCADE_DEPTH = 5;
/** Shade over a tile per step back. */
const CASCADE_SHADE = [0, 0.3, 0.45, 0.58, 0.68, 0.76, 0.8];

export interface CascadeSlot {
  /** Left edge (the tile scales from its left middle). */
  x: number;
  /** Rise toward the horizon. */
  y: number;
  scale: number;
  /** Opacity of the whole tile: 0 for one off screen. */
  opacity: number;
  /** Opacity of the shade drawn over the art. */
  shade: number;
  /** Paint order: the selected tile is in front. */
  z: number;
}

/** Where a tile `offset` places after the selected one sits (negative: before it). */
export function cascadeSlot(detail: DetailLevel, offset: number): CascadeSlot {
  const side = CASCADE[detail].tile;
  const z = 100 - Math.abs(offset);
  if (offset < 0) {
    return { x: CASCADE_LEFT + offset * (side + CASCADE_LEFT + CASCADE_GAP), y: 0, scale: 1, opacity: 0, shade: 0, z };
  }
  let x = CASCADE_LEFT;
  let width = side;
  for (let step = 1; step <= offset; step++) {
    const next = width * CASCADE_SHRINK;
    x += step === 1 ? width + CASCADE_GAP : width - next * CASCADE_OVERLAP;
    width = next;
  }
  return {
    x: Math.round(x),
    y: -2 * offset,
    scale: CASCADE_SHRINK ** offset,
    opacity: offset > CASCADE_DEPTH ? 0 : 1,
    shade: CASCADE_SHADE[Math.min(offset, CASCADE_SHADE.length - 1)],
    z,
  };
}

// Top bar: 16 px of padding, L and R (24 wide, 6 gap each: `PILL_ROOM`)
// around the category names, and the status items at the right in
// `STATUS_W`. Names are at most `TAB_MAX` wide, `TAB_GAP` apart.
export const BAR_SIDE = 16;
export const PILL_ROOM = 24 + 6;
export const STATUS_W = 128;
export const TAB_MAX = 120;
export const TAB_GAP = 16;
