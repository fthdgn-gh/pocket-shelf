import { createEffect, For, Show, untrack } from "solid-js";
import { Grid, Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { focusNode } from "@pocketjs/framework/input";
import { fitTitle } from "../catalog.ts";
import { fontSlot } from "../fonts.ts";
import { TitleArt } from "../components/art.tsx";
import { GRID_COLUMNS, GRID_GAP, GRID_ROWS, GRID_TILE_H } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import type { Game } from "../types.ts";

const ROW_PITCH = GRID_TILE_H + GRID_GAP;

function GridTile(props: {
  game: Game;
  state: LauncherState;
  onPress: () => void;
  ref: (node: NodeMirror) => void;
}) {
  const theme = props.state.theme;
  return (
    <View ref={props.ref} class={theme().tile} focusable onPress={props.onPress}>
      <TitleArt size="md" game={props.game} icon={props.state.icons()[props.game.id]} />
      <Show when={props.state.detail() !== "basic"}>
        <Text class={theme().tileTitle}>{fitTitle(props.game.title, 94, fontSlot(props.state.font(), 12, true))}</Text>
      </Show>
    </View>
  );
}

/** Tiles in rows of GRID_COLUMNS; the grid scrolls one row at a time. */
export function GridView(props: { state: LauncherState }) {
  const { state } = props;
  const tileRefs: (NodeMirror | undefined)[] = [];
  let stripRef: NodeMirror | undefined;

  // First visible row. Starts so the selection is on screen when the view mounts.
  let topRow = Math.max(0, Math.floor(untrack(state.selectedIndex) / GRID_COLUMNS) - GRID_ROWS + 1);

  createEffect(() => {
    state.games(); // re-run when the category changes the list
    const index = state.selectedIndex();
    const row = Math.floor(index / GRID_COLUMNS);
    if (row < topRow) topRow = row;
    else if (row >= topRow + GRID_ROWS) topRow = row - GRID_ROWS + 1;
    const node = tileRefs[index];
    if (node) focusNode(node);
    if (stripRef) animate(stripRef, "translateY", -topRow * ROW_PITCH, { dur: 150, easing: "out" });
  });

  return (
    <View class="w-full grow overflow-hidden items-start justify-center">
      <Grid
        ref={(el) => {
          stripRef = el;
        }}
        gap={GRID_GAP}
        class="flex-row flex-wrap w-[440]"
        style={{ translateY: -topRow * ROW_PITCH }}
      >
        <For each={state.games()}>
          {(game, index) => (
            <GridTile
              game={game}
              state={state}
              onPress={() => state.activate(index())}
              ref={(el) => {
                tileRefs[index()] = el;
              }}
            />
          )}
        </For>
      </Grid>
    </View>
  );
}
