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
export const GRID_TILE = 68;
export const GRID_GAP = 14;
/** Width of a full grid row: columns plus the gaps between them. */
export const GRID_W = GRID_COLUMNS * GRID_TILE + (GRID_COLUMNS - 1) * GRID_GAP;

export const LIST_ROW_H = 30;
export const LIST_GAP = 4;
/** Rows that fit in the list viewport. */
export const LIST_ROWS = 6;

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
// around a clipped strip of fixed-width tabs.
export const TAB_W = 76;
export const TAB_GAP = 4;
export const HEADER_SIDE = 42;
