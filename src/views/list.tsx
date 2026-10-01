import { createEffect, For, Show, untrack } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { fitTitle } from "../catalog.ts";
import { TitleArt } from "../components/art.tsx";
import { LIST_GAP, LIST_ROWS, LIST_ROW_H } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import { fontSlot } from "../text.ts";
import { alpha } from "../themes.ts";
import type { Game } from "../types.ts";

const ROW_PITCH = LIST_ROW_H + LIST_GAP;

// Room for a title in a row: the screen less the margins, the icon and the title id.
const TITLE_W = 300;

function ListRow(props: { game: Game; state: LauncherState; selected: boolean }) {
  const theme = props.state.theme;
  const role = () => (props.selected ? "bodyBold" : "body");
  return (
    <View
      class="relative flex-row shrink-0 items-center gap-2 w-full h-[30] pl-3 pr-3 rounded-lg"
      style={{ bgColor: props.selected ? alpha(theme().accent, "30") : "#00000000" }}
    >
      <Show when={props.selected}>
        <View class="absolute left-0 top-[7] w-[3] h-[16] rounded-[1]" style={{ bgColor: theme().accent }} />
      </Show>
      <TitleArt size="sm" game={props.game} state={props.state} />
      <Text class={props.state.text()[role()]} style={{ textColor: props.selected ? theme().text : theme().dim }}>
        {fitTitle(props.game.title, TITLE_W, fontSlot(props.state.font(), role()))}
      </Text>
      {/* "Detailed" adds the title id at the end of the row. */}
      <Show when={props.state.detail() === "detailed"}>
        <View class="grow" />
        <Text class={props.state.text().small} style={{ textColor: props.selected ? theme().dim : theme().faint }}>
          {props.game.id}
        </Text>
      </Show>
    </View>
  );
}

/** One title per row across the screen; the list scrolls one row at a time. */
export function ListView(props: { state: LauncherState }) {
  const { state } = props;
  let stripRef: NodeMirror | undefined;

  // First visible row. Starts so the selection is on screen when the view mounts.
  let topRow = Math.max(0, untrack(state.selectedIndex) - LIST_ROWS + 1);

  createEffect(() => {
    state.games(); // re-run when the category changes the list
    const index = state.selectedIndex();
    if (index < topRow) topRow = index;
    else if (index >= topRow + LIST_ROWS) topRow = index - LIST_ROWS + 1;
    if (stripRef) animate(stripRef, "translateY", -topRow * ROW_PITCH, { dur: 150, easing: "out" });
  });

  return (
    <View class="flex-row w-full grow px-4">
      <View class="flex-col grow h-[200] mt-[4] overflow-hidden justify-start">
        <View
          ref={(el) => {
            stripRef = el;
          }}
          class="flex-col w-full shrink-0"
          style={{ gap: LIST_GAP, translateY: -topRow * ROW_PITCH }}
        >
          <For each={state.games()}>
            {(game, index) => <ListRow game={game} state={state} selected={state.selectedIndex() === index()} />}
          </For>
        </View>
      </View>
    </View>
  );
}
