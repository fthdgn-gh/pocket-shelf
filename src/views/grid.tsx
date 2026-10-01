import { createEffect, For, Show, untrack } from "solid-js";
import { Grid, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { TitleArt } from "../components/art.tsx";
import { GRID_COLUMNS, GRID_GAP, GRID_ROWS, GRID_TILE } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import type { Game } from "../types.ts";
import { SelectedInfo } from "./info.tsx";

const ROW_PITCH = GRID_TILE + GRID_GAP;

// The selected tile is drawn at its own size, so its frame lands on whole
// pixels and sits evenly around the art. The other tiles are drawn smaller and
// dimmer; they have no frame, so their scaled edges need not be exact.
const TILE = "relative shrink-0 w-[72] h-[72] scale-90 opacity-70 transition duration-150 ease-out";
const TILE_SELECTED = "relative shrink-0 w-[72] h-[72] scale-100 opacity-100 transition duration-150 ease-out";

function GridTile(props: {
  game: Game;
  state: LauncherState;
  selected: boolean;
}) {
  return (
    <View class={props.selected ? TILE_SELECTED : TILE}>
      {/* The selected tile's frame: a filled square behind the art, 3 px larger on each side. */}
      <Show when={props.selected}>
        <View class="absolute inset-[-3]" style={{ bgColor: props.state.theme().accent }} />
      </Show>
      <TitleArt size="md" game={props.game} icon={props.state.icons()[props.game.id]} />
    </View>
  );
}

/** Tiles in rows of GRID_COLUMNS; the grid scrolls one row at a time. */
export function GridView(props: { state: LauncherState }) {
  const { state } = props;
  let stripRef: NodeMirror | undefined;

  // First visible row. Starts so the selection is on screen when the view mounts.
  let topRow = Math.max(0, Math.floor(untrack(state.selectedIndex) / GRID_COLUMNS) - GRID_ROWS + 1);

  createEffect(() => {
    state.games(); // re-run when the category changes the list
    const index = state.selectedIndex();
    const row = Math.floor(index / GRID_COLUMNS);
    if (row < topRow) topRow = row;
    else if (row >= topRow + GRID_ROWS) topRow = row - GRID_ROWS + 1;
    if (stripRef) animate(stripRef, "translateY", -topRow * ROW_PITCH, { dur: 180, easing: "out" });
  });

  return (
    <View class="flex-col items-center w-full grow">
      {/* Two rows, plus room above and below for the selected tile's frame. */}
      <View class="flex-row items-start justify-center w-full h-[168] shrink-0 pt-[6] overflow-hidden">
        <Grid
          ref={(el) => {
            stripRef = el;
          }}
          gap={GRID_GAP}
          class="flex-row flex-wrap w-[400] shrink-0"
          style={{ translateY: -topRow * ROW_PITCH }}
        >
          <For each={state.games()}>
            {(game, index) => (
              <GridTile
                game={game}
                state={state}
                selected={state.selectedIndex() === index()}
              />
            )}
          </For>
        </Grid>
      </View>
      <View class="h-[4] shrink-0" />
      <SelectedInfo state={state} game={state.games()[state.selectedIndex()]} role="title" width={440} />
    </View>
  );
}
