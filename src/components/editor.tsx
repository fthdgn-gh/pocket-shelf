import { For } from "solid-js";
import { fitTitle } from "../catalog.ts";
import type { LauncherState } from "../state.ts";
import { fontSlot } from "../text.ts";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

// Room for a value beside its label in a wide drawer.
const VALUE_W = 170;

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
      { label: "Category", value: state.editorRow() === 0 ? `◄ ${category} ►` : category },
      { label: "Title", value: fit(game?.title ?? "") },
      { label: "Box art", value: fit(art()) },
      { label: "Reset to defaults", value: "" },
    ];
  };
  return (
    <Drawer state={state} title="Edit title" size="wide">
      <For each={rows()}>
        {(row, index) => (
          <OptionRow state={state} active={state.editorRow() === index()} label={row.label} value={row.value} />
        )}
      </For>
      <Spacer />
      <Note state={state}>
        {state.editorNote() || `${state.confirmGlyph()}: Change  ·  ${state.cancelGlyph()}: Back`}
      </Note>
    </Drawer>
  );
}
