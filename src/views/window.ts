import { createMemo } from "solid-js";
import type { LauncherState } from "../state.ts";
import type { Game } from "../types.ts";

/**
 * The part of the current category a view keeps on screen. A category can
 * hold hundreds of titles; a view mounts the ones in `range` (indexes `from`
 * up to, not including, `to`) and places each by its index in the full list,
 * so the rest cost nothing.
 */
export function createWindow(state: LauncherState, range: () => [from: number, to: number]) {
  // Index of every title in the full list, rebuilt when the list changes.
  const positions = createMemo(() => new Map(state.games().map((game, index) => [game, index])));
  const items = createMemo(() => {
    const [from, to] = range();
    return state.games().slice(Math.max(0, from), Math.max(0, to));
  });
  return {
    /** The titles to mount. `For` keeps the nodes of titles that stay in range. */
    items,
    indexOf: (game: Game): number => positions().get(game) ?? 0,
  };
}
