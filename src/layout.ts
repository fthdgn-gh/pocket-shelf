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

// Category header: a badge on each side (24 wide, 12 from the edge, 6 gap)
// around a clipped strip of fixed-width tabs. A tab holds the category's name
// and, after `TAB_COUNT_GAP`, its number of titles in a badge with
// `TAB_COUNT_PAD` on each side of the digits.
export const TAB_W = 112;
export const TAB_COUNT_GAP = 4;
export const TAB_COUNT_PAD = 4;
export const TAB_GAP = 4;
export const HEADER_SIDE = 42;
