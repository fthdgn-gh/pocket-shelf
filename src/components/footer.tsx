import { Show } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import type { LauncherState } from "../state.ts";
import { fitTitle } from "../catalog.ts";
import { fill } from "../i18n.ts";
import { fontSlot } from "../text.ts";
import { hints, Prompt } from "./icons.tsx";

// Room for the search term beside the position. The "Menu" hint leaves the
// legend on the search tab to make this room.
const TERM_W = 96;

/**
 * Button legend on the left, position in the current category on the right.
 * On the search tab the search term stands before the position.
 */
export function Footer(props: { state: LauncherState }) {
  const { state } = props;
  const position = () => (state.games().length === 0 ? 0 : state.selectedIndex() + 1);
  return (
    <View class="flex-row items-center justify-between w-full h-[26] shrink-0 px-4">
      <Show
        when={state.launchingTitle()}
        fallback={
          <Prompt
            state={state}
            parts={hints(
              [state.confirmButton(), state.t().launch],
              ["triangle", state.t().edit],
              ["square", state.t().search],
              state.searching() ? [state.cancelButton(), state.t().close] : ["select", state.t().menu],
            )}
          />
        }
      >
        <Text class={state.text().smallBold} style={{ textColor: state.theme().accent }}>
          {fill(state.t().launching, { title: state.launchingTitle() ?? "" })}
        </Text>
      </Show>
      <View class="flex-row items-center gap-2 shrink-0">
        <Show when={state.searching()}>
          <Text class={state.text().smallBold} style={{ textColor: state.theme().accent }}>
            {`"${fitTitle(state.searchTerm(), TERM_W, fontSlot(state.font(), "smallBold"))}"`}
          </Text>
        </Show>
        <Text class={state.text().small} style={{ textColor: state.theme().faint }}>
          {`${position()} / ${state.games().length}`}
        </Text>
      </View>
    </View>
  );
}
