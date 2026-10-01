import { createEffect, createSignal, For, Show, untrack } from "solid-js";
import { View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { SelectedFrame, TitleArt } from "../components/art.tsx";
import { GRID_COLUMNS, GRID_GAP, GRID_ROWS, GRID_TILE } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import type { Game } from "../types.ts";
import { SelectedInfo } from "./info.tsx";
import { createWindow } from "./window.ts";

const ROW_PITCH = GRID_TILE + GRID_GAP;

// The selected tile is drawn at its own size, so its frame lands on whole
// pixels and sits evenly around the art. The other tiles are drawn smaller and
// dimmer; they have no frame, so their scaled edges need not be exact. A tile
// is placed by its row and column, so only the rows near the screen are mounted.
const TILE = "absolute w-[72] h-[72] scale-90 opacity-70 transition duration-150 ease-out";
const TILE_SELECTED = "absolute w-[72] h-[72] scale-100 opacity-100 transition duration-150 ease-out";

function GridTile(props: { game: Game; state: LauncherState; selected: boolean; index: number }) {
  return (
    <View
      class={props.selected ? TILE_SELECTED : TILE}
      style={{
        insetL: (props.index % GRID_COLUMNS) * ROW_PITCH,
        insetT: Math.floor(props.index / GRID_COLUMNS) * ROW_PITCH,
      }}
    >
      <Show when={props.selected}>
        <SelectedFrame state={props.state} />
      </Show>
      <TitleArt size="md" game={props.game} state={props.state} />
    </View>
  );
}

/** Tiles in rows of GRID_COLUMNS; the grid scrolls one row at a time. */
export function GridView(props: { state: LauncherState }) {
  const { state } = props;
  let stripRef: NodeMirror | undefined;

  // First visible row. Starts so the selection is on screen when the view mounts.
  const [topRow, setTopRow] = createSignal(
    Math.max(0, Math.floor(untrack(state.selectedIndex) / GRID_COLUMNS) - GRID_ROWS + 1),
  );
  // The visible rows and one more above and below, which a scroll slides through.
  const grid = createWindow(state, () => [
    (topRow() - 1) * GRID_COLUMNS,
    (topRow() + GRID_ROWS + 1) * GRID_COLUMNS,
  ]);

  createEffect(() => {
    state.games(); // re-run when the category changes the list
    const row = Math.floor(state.selectedIndex() / GRID_COLUMNS);
    const top = untrack(topRow);
    if (row < top) setTopRow(row);
    else if (row >= top + GRID_ROWS) setTopRow(row - GRID_ROWS + 1);
    if (stripRef) animate(stripRef, "translateY", -untrack(topRow) * ROW_PITCH, { dur: 180, easing: "out" });
  });

  return (
    <View class="flex-col items-center w-full grow">
      {/* Two rows, plus room above and below for the selected tile's frame. */}
      <View class="flex-row items-start justify-center w-full h-[168] shrink-0 pt-[6] overflow-hidden">
        <View
          ref={(el) => {
            stripRef = el;
          }}
          class="relative w-[400] h-[154] shrink-0"
          style={{ translateY: -untrack(topRow) * ROW_PITCH }}
        >
          <For each={grid.items()}>
            {(game) => (
              <GridTile
                game={game}
                state={state}
                selected={state.selectedIndex() === grid.indexOf(game)}
                index={grid.indexOf(game)}
              />
            )}
          </For>
        </View>
      </View>
      <View class="h-[4] shrink-0" />
      <SelectedInfo state={state} game={state.games()[state.selectedIndex()]} role="title" width={440} />
    </View>
  );
}
