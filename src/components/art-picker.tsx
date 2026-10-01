import { For } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import { artFolder } from "../art-files.ts";
import { PICKER_ROWS, type LauncherState } from "../state.ts";

/** Lists the PNG files in the art folder, after a "Default" entry. */
export function ArtPickerOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const entries = () => ["Default", "Game icon", ...state.artFiles()];
  // Scroll so the highlighted row stays in view.
  const first = () => {
    const last = entries().length - PICKER_ROWS;
    return Math.max(0, Math.min(state.artRow() - Math.floor(PICKER_ROWS / 2), last));
  };
  const visible = () => entries().slice(first(), first() + PICKER_ROWS);
  const message = () =>
    state.artNote() ||
    (state.artFiles().length === 0
      ? `No PNG files yet. Copy images to ${artFolder()}`
      : `${state.confirmGlyph()}: Use  ·  ${state.cancelGlyph()}: Back`);
  return (
    <View class="absolute inset-0 items-center justify-center bg-[#000000e6]">
      <View class={state.theme().panelWide}>
        <Text class={state.theme().menuTitle}>Box art</Text>
        <For each={visible()}>
          {(name, index) => (
            <View
              class={
                first() + index() === state.artRow()
                  ? state.theme().menuRowActive
                  : state.theme().menuRow
              }
            >
              <Text class={state.theme().menuLabel}>{name.length > 40 ? `${name.slice(0, 37)}...` : name}</Text>
            </View>
          )}
        </For>
        <Text class={state.theme().menuHint}>{message()}</Text>
      </View>
    </View>
  );
}
