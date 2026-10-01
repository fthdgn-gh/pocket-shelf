import { createEffect, For, Show, untrack } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { focusNode } from "@pocketjs/framework/input";
import { fitTitle } from "../catalog.ts";
import { fontSlot } from "../fonts.ts";
import { TitleArt } from "../components/art.tsx";
import { LIST_GAP, LIST_ROWS, LIST_ROW_H } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import type { Game } from "../types.ts";

const ROW_PITCH = LIST_ROW_H + LIST_GAP;

function ListRow(props: {
  game: Game;
  state: LauncherState;
  onPress: () => void;
  ref: (node: NodeMirror) => void;
}) {
  const theme = props.state.theme;
  return (
    <View ref={props.ref} class={theme().row} focusable onPress={props.onPress}>
      <TitleArt size="sm" game={props.game} icon={props.state.icons()[props.game.id]} />
      {/* The list always shows the title; Basic only drops the title id. */}
      <Text class={theme().rowTitle}>{fitTitle(props.game.title, 290, fontSlot(props.state.font(), 14, true))}</Text>
      <Show when={props.state.detail() === "detailed"}>
        <Text class={theme().cardId}>{props.game.id}</Text>
      </Show>
    </View>
  );
}

/** One title per row; the list scrolls one row at a time. */
export function ListView(props: { state: LauncherState }) {
  const { state } = props;
  const rowRefs: (NodeMirror | undefined)[] = [];
  let stripRef: NodeMirror | undefined;

  // First visible row. Starts so the selection is on screen when the view mounts.
  let topRow = Math.max(0, untrack(state.selectedIndex) - LIST_ROWS + 1);

  createEffect(() => {
    state.games(); // re-run when the category changes the list
    const index = state.selectedIndex();
    if (index < topRow) topRow = index;
    else if (index >= topRow + LIST_ROWS) topRow = index - LIST_ROWS + 1;
    const node = rowRefs[index];
    if (node) focusNode(node);
    if (stripRef) animate(stripRef, "translateY", -topRow * ROW_PITCH, { dur: 150, easing: "out" });
  });

  return (
    <View class="w-full grow overflow-hidden items-start justify-start px-5">
      <View
        ref={(el) => {
          stripRef = el;
        }}
        class="flex-col w-full"
        style={{ gap: LIST_GAP, translateY: -topRow * ROW_PITCH }}
      >
        <For each={state.games()}>
          {(game, index) => (
            <ListRow
              game={game}
              state={state}
              onPress={() => state.activate(index())}
              ref={(el) => {
                rowRefs[index()] = el;
              }}
            />
          )}
        </For>
      </View>
    </View>
  );
}
