import { Show } from "solid-js";
import { glyph } from "@pocketjs/framework/modality";
import { Text, View } from "@pocketjs/framework/components";
import type { LauncherState } from "../state.ts";

export function Footer(props: { state: LauncherState }) {
  const { state } = props;
  // Shown only while a launch is in progress.
  const message = () => {
    const launching = state.launchingTitle();
    return launching ? `Launching ${launching}...` : "";
  };
  const position = () => (state.games().length === 0 ? 0 : state.selectedIndex() + 1);
  const hints = () => `${state.confirmGlyph()}: Launch  ·  ${glyph("triangle")}: Edit  ·  SELECT: Menu`;
  return (
    <View class="flex-col px-5 py-1 gap-1">
      <Show when={message()}>
        <Text class={state.theme().footerAccent}>{message()}</Text>
      </Show>
      <View class="flex-row items-center justify-between">
        <Text class={state.theme().footerDim}>{hints()}</Text>
        <View class={state.theme().pill}>
          <Text class={state.theme().pillText}>
            {position()} / {state.games().length}
          </Text>
        </View>
      </View>
    </View>
  );
}
