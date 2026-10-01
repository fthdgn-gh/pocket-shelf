import { For } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import type { LauncherState } from "../state.ts";

/** Per-title editor: category, title, box art and reset. */
export function EditorOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const rows = () => {
    const game = state.editorGame();
    return [
      {
        label: "Category",
        value: `◄ ${state.categoryLabel(game?.category ?? "")} ►`,
      },
      { label: "Title", value: game?.title ?? "" },
      { label: "Box art", value: game?.artIcon ? "Game icon" : game?.art ? (game.artAuto ? `${game.art} (auto)` : game.art) : "Default" },
      { label: "Reset to defaults", value: "" },
    ];
  };
  const hint = () => `${state.confirmGlyph()}: Change  ·  ${state.cancelGlyph()}: Back`;
  return (
    <View class="absolute inset-0 items-center justify-center bg-[#000000b3]">
      <View class={state.theme().menuPanel}>
        <Text class={state.theme().menuTitle}>Edit title</Text>
        <For each={rows()}>
          {(row, index) => (
            <View class={state.editorRow() === index() ? state.theme().menuRowActive : state.theme().menuRow}>
              <Text class={state.theme().menuLabel}>{row.label}</Text>
              <Text class={state.theme().menuValue}>{row.value}</Text>
            </View>
          )}
        </For>
        <Text class={state.theme().menuHint}>{state.editorNote() || hint()}</Text>
      </View>
    </View>
  );
}
