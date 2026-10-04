import { createEffect, createSignal, For, Show, untrack } from "solid-js";
import { View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { SelectedFrame, TitleArt, type ArtSize } from "../components/art.tsx";
import { GRID, GRID_COLUMNS, GRID_ROWS } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import type { DetailLevel, Game } from "../types.ts";
import { SelectedInfo } from "./info.tsx";
import { createWindow } from "./window.ts";

/** Distance from one row (or column) to the next. */
const pitchOf = (detail: DetailLevel) => GRID[detail].tile + GRID[detail].gap;

// Class literals per detail level; the sizes match GRID in layout.ts. The
// selected tile is drawn at its own size, so its frame lands on whole pixels
// and sits evenly around the art. The other tiles are drawn smaller and
// dimmer; they have no frame, so their scaled edges need not be exact. A tile
// is placed by its row and column, so only the rows near the screen are mounted.
// `area` holds two rows plus 6 px above and 8 below for the selected frame;
// `strip` is two rows and the gap between them, five columns wide.
const LARGE = {
  art: "md",
  tile: "absolute w-[72] h-[72] scale-90 opacity-70 transition duration-150 ease-out",
  selected: "absolute w-[72] h-[72] scale-100 opacity-100 transition duration-150 ease-out",
  area: "flex-row items-start justify-center w-full h-[168] shrink-0 pt-[6] overflow-hidden",
  strip: "relative w-[400] h-[154] shrink-0",
} as const;
const SHAPES: Record<DetailLevel, { art: ArtSize; tile: string; selected: string; area: string; strip: string }> = {
  basic: LARGE,
  normal: LARGE,
  detailed: {
    art: "mdDetailed",
    tile: "absolute w-[64] h-[64] scale-90 opacity-70 transition duration-150 ease-out",
    selected: "absolute w-[64] h-[64] scale-100 opacity-100 transition duration-150 ease-out",
    area: "flex-row items-start justify-center w-full h-[152] shrink-0 pt-[6] overflow-hidden",
    strip: "relative w-[360] h-[138] shrink-0",
  },
};

function GridTile(props: { game: Game; state: LauncherState; selected: boolean; index: number }) {
  const shape = () => SHAPES[props.state.detail()];
  const pitch = () => pitchOf(props.state.detail());
  return (
    <View
      class={props.selected ? shape().selected : shape().tile}
      style={{
        insetL: (props.index % GRID_COLUMNS) * pitch(),
        insetT: Math.floor(props.index / GRID_COLUMNS) * pitch(),
      }}
    >
      <Show when={props.selected}>
        <SelectedFrame state={props.state} />
      </Show>
      <TitleArt size={shape().art} game={props.game} state={props.state} />
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
    const pitch = pitchOf(state.detail()); // and when the detail level changes the tile size
    const row = Math.floor(state.selectedIndex() / GRID_COLUMNS);
    const top = untrack(topRow);
    if (row < top) setTopRow(row);
    else if (row >= top + GRID_ROWS) setTopRow(row - GRID_ROWS + 1);
    if (stripRef) animate(stripRef, "translateY", -untrack(topRow) * pitch, { dur: 180, easing: "out" });
  });

  return (
    <View class="flex-col items-center w-full grow">
      <View class={SHAPES[state.detail()].area}>
        <View
          ref={(el) => {
            stripRef = el;
          }}
          class={SHAPES[state.detail()].strip}
          style={{ translateY: untrack(() => -topRow() * pitchOf(state.detail())) }}
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
