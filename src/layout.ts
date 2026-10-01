// Geometry shared by the views. The class literals in themes.ts spell the same
// sizes (for example `w-[150]`), so change both together.

export const CARD_W = 150;
export const CARD_GAP = 16;

export const GRID_COLUMNS = 4;
export const GRID_ROWS = 2;
export const GRID_TILE_W = 104;
export const GRID_TILE_H = 84;
export const GRID_GAP = 8;
/** Width of a full grid row: columns plus the gaps between them. */
export const GRID_W = GRID_COLUMNS * GRID_TILE_W + (GRID_COLUMNS - 1) * GRID_GAP;

export const LIST_ROW_H = 34;
export const LIST_GAP = 6;
/** Rows that fit fully in the list viewport; the next row shows partly. */
export const LIST_ROWS = 4;

export interface CarouselLayout {
  cardPitch: number;
  /** translateX that centers the first card in a viewport `screenW` wide. */
  centerX: number;
}

export function carouselLayout(screenW: number): CarouselLayout {
  return {
    cardPitch: CARD_W + CARD_GAP,
    centerX: Math.round((screenW - CARD_W) / 2),
  };
}

// Category header: a badge on each side (28 wide, 20 from the edge, 8 gap)
// around a clipped strip of fixed-width tabs.
export const TAB_W = 96;
export const TAB_GAP = 8;
export const HEADER_SIDE = 56;
