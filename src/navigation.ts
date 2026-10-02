import type { FocusDirection } from "@pocketjs/framework/input";
import { GRID_COLUMNS, GRID_ROWS, LIST_ROWS } from "./layout.ts";
import type { DetailLevel, ViewMode } from "./types.ts";

export const VIEW_MODES: readonly ViewMode[] = ["carousel", "grid", "list"];
export const DETAIL_LEVELS: readonly DetailLevel[] = ["basic", "normal", "detailed"];

/** Items the L and R triggers jump over in each view. */
export function pageSize(view: ViewMode): number {
  if (view === "grid") return GRID_COLUMNS * GRID_ROWS;
  if (view === "list") return LIST_ROWS;
  return 1;
}

/** How many items around the selection get their icons loaded. */
export function iconRadius(view: ViewMode): number {
  if (view === "grid") return GRID_COLUMNS * 2;
  if (view === "list") return LIST_ROWS + 2;
  // Two full tiles show on each side of the selected one, and part of a third.
  return 3;
}

/**
 * The index a D-pad press selects, or null when the press goes nowhere
 * (an edge of the list, or an axis the view does not use).
 */
export function moveSelection(
  view: ViewMode,
  direction: FocusDirection,
  index: number,
  count: number,
): number | null {
  const step = direction === "left" || direction === "up" ? -1 : 1;
  const horizontal = direction === "left" || direction === "right";
  let next: number | null = null;
  if (view === "carousel") {
    next = horizontal ? index + step : null;
  } else if (view === "list") {
    next = horizontal ? null : index + step;
  } else if (horizontal) {
    next = index + step;
  } else {
    next = index + step * GRID_COLUMNS;
    // A partial last row still holds a destination below a longer column.
    if (next >= count && Math.floor(index / GRID_COLUMNS) < Math.floor((count - 1) / GRID_COLUMNS)) {
      next = count - 1;
    }
  }
  if (next === null || next < 0 || next >= count) return null;
  return next;
}
