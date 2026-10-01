import { Show } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import type { LauncherState } from "../state.ts";
import { hints, Prompt } from "./icons.tsx";

/** Button legend on the left, position in the current category on the right. */
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
            parts={hints([state.confirmButton(), "Launch"], ["triangle", "Edit"], ["select", "Menu"])}
          />
        }
      >
        <Text class={state.text().smallBold} style={{ textColor: state.theme().accent }}>
          {`Launching ${state.launchingTitle()}...`}
        </Text>
      </Show>
      <Text class={state.text().small} style={{ textColor: state.theme().faint }}>
        {`${position()} / ${state.games().length}`}
      </Text>
    </View>
  );
}
