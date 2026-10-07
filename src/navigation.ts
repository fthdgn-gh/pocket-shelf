import type { FocusDirection } from "@pocketjs/framework/input";
import { CASCADE_DEPTH, GRID_COLUMNS, GRID_ROWS, LIST_ROWS, listRows } from "./layout.ts";
import type { DetailLevel, ViewMode } from "./types.ts";

export const VIEW_MODES: readonly ViewMode[] = ["carousel", "grid", "list", "cascade"];
export const DETAIL_LEVELS: readonly DetailLevel[] = ["basic", "normal", "detailed"];

/** Items the L and R triggers jump over in each view. The list shows a row less under the status bar. */
export function pageSize(view: ViewMode, statusBar = false): number {
  if (view === "grid") return GRID_COLUMNS * GRID_ROWS;
  if (view === "list") return listRows(statusBar);
  return 1;
}

/** How many items around the selection get their icons loaded. */
export function iconRadius(view: ViewMode): number {
  if (view === "grid") return GRID_COLUMNS * 2;
  if (view === "list") return LIST_ROWS + 2;
  // The cascade shows its tiles after the selected one only.
  if (view === "cascade") return CASCADE_DEPTH + 1;
  // Two full tiles show on each side of the selected one, and part of a third.
  return 3;
}

/**
 * The index a D-pad press selects, or null when the press goes nowhere
 * (an edge of the list, or an axis the view does not use).
 *
 * With `wrap`, a press past an edge continues at the other one: after the
 * last item comes the first, and in the grid a press up from the first row
 * lands in the same column of the last row.
 */
export function moveSelection(
  view: ViewMode,
  direction: FocusDirection,
  index: number,
  count: number,
  wrap = false,
): number | null {
  const step = direction === "left" || direction === "up" ? -1 : 1;
  const horizontal = direction === "left" || direction === "right";
  let next: number | null = null;
  if (view === "carousel" || view === "cascade") {
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
  if (next === null) return null;
  if (next >= 0 && next < count) return next;
  if (!wrap || count < 2) return null;
  if (view !== "grid" || horizontal) return next < 0 ? count - 1 : 0;
  // Grid, up or down: the same column at the other end. A last row too short
  // to have that column gives its last item.
  const column = index % GRID_COLUMNS;
  const lastRow = Math.floor((count - 1) / GRID_COLUMNS) * GRID_COLUMNS;
  if (lastRow === 0) return null;
  return next < 0 ? Math.min(lastRow + column, count - 1) : column;
}
