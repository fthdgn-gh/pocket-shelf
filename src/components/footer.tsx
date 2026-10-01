import { For, Show } from "solid-js";
import { glyph } from "@pocketjs/framework/modality";
import { Text, View } from "@pocketjs/framework/components";
import type { LauncherState } from "../state.ts";

/** One button hint: the button's glyph in the accent color, then what it does. */
export function Hint(props: { state: LauncherState; button: string; label: string }) {
  return (
    <View class="flex-row items-center gap-1 shrink-0">
      <Text class={props.state.text().smallBold} style={{ textColor: props.state.theme().accent }}>
        {props.button}
      </Text>
      <Text class={props.state.text().small} style={{ textColor: props.state.theme().dim }}>
        {props.label}
      </Text>
    </View>
  );
}

/** Button legend on the left, position in the current category on the right. */
export function Footer(props: { state: LauncherState }) {
  const { state } = props;
  const hints = () => [
    { button: state.confirmGlyph(), label: "Launch" },
    { button: glyph("triangle"), label: "Edit" },
    { button: "SELECT", label: "Menu" },
  ];
  const position = () => (state.games().length === 0 ? 0 : state.selectedIndex() + 1);
  return (
    <View class="flex-row items-center justify-between w-full h-[26] shrink-0 px-4">
      <Show
        when={state.launchingTitle()}
        fallback={
          <View class="flex-row items-center gap-3">
            <For each={hints()}>{(hint) => <Hint state={state} button={hint.button} label={hint.label} />}</For>
          </View>
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
