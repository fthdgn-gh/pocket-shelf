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

function ListRow(props: {
  game: Game;
  state: LauncherState;
  selected: boolean;
}) {
  const theme = props.state.theme;
  return (
    <View
      class="relative flex-row shrink-0 items-center gap-2 w-full h-[30] pl-3 pr-2 rounded-lg"
      style={{ bgColor: props.selected ? alpha(theme().accent, "30") : "#00000000" }}
    >
      <Show when={props.selected}>
        <View class="absolute left-0 top-[7] w-[3] h-[16] rounded-[1]" style={{ bgColor: theme().accent }} />
      </Show>
      <TitleArt size="sm" game={props.game} icon={props.state.icons()[props.game.id]} />
      <Text
        class={props.selected ? props.state.text().bodyBold : props.state.text().body}
        style={{ textColor: props.selected ? theme().text : theme().dim }}
      >
        {fitTitle(props.game.title, 196, fontSlot(props.state.font(), props.selected ? "bodyBold" : "body"))}
      </Text>
    </View>
  );
}

/** The selected title at full size, beside the list. */
function Detail(props: { state: LauncherState; game: Game }) {
  const { state } = props;
  return (
    <View class="flex-col items-center justify-center gap-2 grow h-full">
      <TitleArt size="lg" game={props.game} icon={state.icons()[props.game.id]} />
      <Show when={state.detail() !== "basic"}>
        <View class="flex-col items-center gap-[2]">
          <Text class={state.text().title} style={{ textColor: state.theme().text }}>
            {fitTitle(props.game.title, 176, fontSlot(state.font(), "title"))}
          </Text>
          <Show when={state.detail() === "detailed"}>
            <Text class={state.text().small} style={{ textColor: state.theme().dim }}>
              {`${props.game.id}  ·  ${state.categoryLabel(props.game.category)}`}
            </Text>
          </Show>
        </View>
      </Show>
    </View>
  );
}

/** Titles by name on the left, the selected one in detail on the right. */
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
    <View class="flex-row w-full grow pl-4 pr-2">
      <View class="flex-col w-[250] h-[200] shrink-0 mt-[4] overflow-hidden justify-start">
        <View
          ref={(el) => {
            stripRef = el;
          }}
          class="flex-col w-full shrink-0"
          style={{ gap: LIST_GAP, translateY: -topRow * ROW_PITCH }}
        >
          <For each={state.games()}>
            {(game, index) => (
              <ListRow
                game={game}
                state={state}
                selected={state.selectedIndex() === index()}
              />
            )}
          </For>
        </View>
      </View>
      <Show when={state.games()[state.selectedIndex()]}>{(game) => <Detail state={state} game={game()} />}</Show>
    </View>
  );
}
