import type { FocusDirection } from "@pocketjs/framework/input";
import { CASCADE_DEPTH, GRID_COLUMNS, GRID_ROWS, HOME_AFTER, LIST_ROWS, listRows } from "./layout.ts";
import type { DetailLevel, ViewMode } from "./types.ts";

export const VIEW_MODES: readonly ViewMode[] = ["carousel", "grid", "list", "cascade", "home"];
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
  // Home shows the selected tile and the ones after it.
  if (view === "home") return HOME_AFTER;
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
  if (view === "carousel" || view === "cascade" || view === "home") {
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

/**
 * How far one repeat of a held direction moves on the shelf: one item at
 * first, then five after the direction has been held 1.2 s, then twenty after
 * 3 s, so a list of thousands can be crossed by holding. Counted in host
 * frames (60 per second) since the press.
 */
export const HOLD_STAGES: readonly { after: number; steps: number }[] = [
  { after: 0, steps: 1 },
  { after: 72, steps: 5 },
  { after: 180, steps: 20 },
];

/** Items a repeat moves after the direction has been held `heldFrames`. */
export function holdSteps(heldFrames: number): number {
  let steps = 1;
  for (const stage of HOLD_STAGES) if (heldFrames >= stage.after) steps = stage.steps;
  return steps;
}

/**
 * The index `steps` moves away in `direction`, without wrapping: it stops at
 * the last item the direction reaches. Null when not even one move is
 * possible.
 */
export function moveSelectionBy(
  view: ViewMode,
  direction: FocusDirection,
  index: number,
  count: number,
  steps: number,
): number | null {
  let current: number | null = null;
  for (let step = 0; step < steps; step++) {
    const next = moveSelection(view, direction, current ?? index, count);
    if (next === null) break;
    current = next;
  }
  return current;
}
