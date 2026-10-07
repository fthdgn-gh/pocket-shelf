// Geometry shared by the views. The class literals in views/ and components/
// spell the same sizes (for example `w-[84]`), so change both together.

import type { DetailLevel } from "./types.ts";

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
/** Rows that fit while the status bar takes the top of the screen. */
export const LIST_ROWS_UNDER_STATUS = 5;

/** List rows that fit, with or without the status bar. */
export function listRows(statusBar: boolean): number {
  return statusBar ? LIST_ROWS_UNDER_STATUS : LIST_ROWS;
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

// Category header: a badge on each side (24 wide, 12 from the edge, 6 gap)
// around a clipped strip of fixed-width tabs. A tab holds the category's name
// and, after `TAB_COUNT_GAP`, its number of titles in a badge with
// `TAB_COUNT_PAD` on each side of the digits.
export const TAB_W = 112;
export const TAB_COUNT_GAP = 4;
export const TAB_COUNT_PAD = 4;
export const TAB_GAP = 4;
export const HEADER_SIDE = 42;
