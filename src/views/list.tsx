import { createEffect, createSignal, For, Show, untrack } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { fitTitle } from "../catalog.ts";
import { TitleArt } from "../components/art.tsx";
import { LIST_GAP, LIST_ROW_H, listRows } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import { fontSlot } from "../text.ts";
import { alpha } from "../themes.ts";
import type { Game } from "../types.ts";
import { createWindow } from "./window.ts";

const ROW_PITCH = LIST_ROW_H + LIST_GAP;

// Room for a title in a row: the screen less the margins, the icon and the title id.
const TITLE_W = 300;

function ListRow(props: { game: Game; state: LauncherState; selected: boolean; index: number }) {
  const theme = props.state.theme;
  const role = () => (props.selected ? "bodyBold" : "body");
  return (
    <View
      class="absolute left-0 right-0 flex-row items-center gap-2 h-[30] pl-3 pr-3 rounded-lg"
      style={{
        insetT: props.index * ROW_PITCH,
        bgColor: props.selected ? alpha(theme().accent, "30") : "#00000000",
      }}
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

/**
 * One title per row across the screen; the list scrolls one row at a time. A
 * row is placed by its index, so only the rows near the screen are mounted.
 */
export function ListView(props: { state: LauncherState }) {
  const { state } = props;
  let stripRef: NodeMirror | undefined;

  // Rows on screen: one less while the status bar takes the top of the screen.
  const rows = () => listRows(state.statusBarOn());
  // First visible row. Starts so the selection is on screen when the view mounts.
  const [topRow, setTopRow] = createSignal(Math.max(0, untrack(state.selectedIndex) - untrack(rows) + 1));
  // The visible rows and two more above and below, which a scroll slides through.
  const list = createWindow(state, () => [topRow() - 2, topRow() + rows() + 2]);

  createEffect(() => {
    state.games(); // re-run when the category changes the list
    const index = state.selectedIndex();
    const top = untrack(topRow);
    if (index < top) setTopRow(index);
    else if (index >= top + rows()) setTopRow(index - rows() + 1);
    if (stripRef) animate(stripRef, "translateY", -untrack(topRow) * ROW_PITCH, { dur: 150, easing: "out" });
  });

  return (
    <View class="flex-row w-full grow px-4">
      {/* Six rows of 30 with gaps of 4 are 200 high; five are 166. */}
      <View
        class={
          state.statusBarOn()
            ? "flex-col grow h-[166] mt-[4] overflow-hidden justify-start"
            : "flex-col grow h-[200] mt-[4] overflow-hidden justify-start"
        }
      >
        <View
          ref={(el) => {
            stripRef = el;
          }}
          class="relative w-full h-[200] shrink-0"
          style={{ translateY: -untrack(topRow) * ROW_PITCH }}
        >
          <For each={list.items()}>
            {(game) => (
              <ListRow
                game={game}
                state={state}
                selected={state.selectedIndex() === list.indexOf(game)}
                index={list.indexOf(game)}
              />
            )}
          </For>
        </View>
      </View>
    </View>
  );
}
