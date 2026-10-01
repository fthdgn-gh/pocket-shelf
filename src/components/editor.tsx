import { For, Show } from "solid-js";
import { fitTitle } from "../catalog.ts";
import type { LauncherState } from "../state.ts";
import { fontSlot } from "../text.ts";
import { hints, Prompt } from "./icons.tsx";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

// Room for a value beside its label in a wide drawer, with its arrow.
const VALUE_W = 160;

/** Per-title editor: category, title, box art and reset. */
export function EditorOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const fit = (value: string) => fitTitle(value, VALUE_W, fontSlot(state.font(), "bodyBold"));
  const art = () => {
    const game = state.editorGame();
    if (game?.artIcon) return "Game icon";
    if (!game?.art) return "Default";
    return game.artAuto ? `${game.art} (auto)` : game.art;
  };
  const rows = () => {
    const game = state.editorGame();
    const category = state.categoryLabel(game?.category ?? "");
    return [
      { label: "Category", value: category, step: "both" as const },
      { label: "Title", value: fit(game?.title ?? ""), step: "right" as const },
      { label: "Box art", value: fit(art()), step: "right" as const },
      { label: "Reset to defaults", value: "", step: undefined },
    ];
  };
  return (
    <Drawer state={state} title="Edit title" size="wide">
      <For each={rows()}>
        {(row, index) => (
          <OptionRow
            state={state}
            active={state.editorRow() === index()}
            label={row.label}
            value={row.value}
            step={row.step}
          />
        )}
      </For>
      <Spacer />
      <Show
        when={state.editorNote()}
        fallback={<Prompt state={state} parts={hints([state.confirmButton(), "Change"], [state.cancelButton(), "Back"])} />}
      >
        <Note state={state}>{state.editorNote()}</Note>
      </Show>
    </Drawer>
  );
}
