import { For } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import { glyph } from "@pocketjs/framework/modality";
import { PICKER_ROWS, type LauncherState } from "../state.ts";

/** Create, rename, delete, reorder and hide categories. */
export function CategoryManagerOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const entries = () => [
    { label: "New category", value: "+" },
    ...state.allCategories().map((item) => ({
      label: item.label,
      value: state.isHidden(item.id) ? "Hidden" : "",
    })),
  ];
  // Scroll so the highlighted row stays in view.
  const first = () => {
    const last = entries().length - PICKER_ROWS;
    return Math.max(0, Math.min(state.catRow() - Math.floor(PICKER_ROWS / 2), last));
  };
  const visible = () => entries().slice(first(), first() + PICKER_ROWS);
  const hintEdit = () => `${state.confirmGlyph()}: Rename  ${glyph("triangle")}: New  ${glyph("square")}: Delete`;
  const hintMove = () => `START: Hide / Show  L R: Move  ${state.cancelGlyph()}: Back`;
  return (
    <View class="absolute inset-0 items-center justify-center bg-[#000000e6]">
      <View class={state.theme().panelWide}>
        <Text class={state.theme().menuTitle}>Categories</Text>
        <For each={visible()}>
          {(entry, index) => (
            <View
              class={
                first() + index() === state.catRow()
                  ? state.theme().menuRowActive
                  : state.theme().menuRow
              }
            >
              <Text class={state.theme().menuLabel}>{entry.label}</Text>
              <Text class={state.theme().menuValue}>{entry.value}</Text>
            </View>
          )}
        </For>
        <Text class={state.theme().menuHint}>{state.catNote() || " "}</Text>
        <Text class={state.theme().menuHint}>{hintEdit()}</Text>
        <Text class={state.theme().menuHint}>{hintMove()}</Text>
      </View>
    </View>
  );
}
