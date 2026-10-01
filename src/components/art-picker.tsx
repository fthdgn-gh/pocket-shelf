import { For } from "solid-js";
import { artFolder } from "../art-files.ts";
import { fitTitle } from "../catalog.ts";
import { PICKER_ROWS, type LauncherState } from "../state.ts";
import { fontSlot } from "../text.ts";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

/** Lists the PNG files in the art folder, after "Default" and "Game icon". */
export function ArtPickerOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const entries = () => ["Default", "Game icon", ...state.artFiles()];
  // Scroll so the highlighted row stays in view.
  const first = () => {
    const last = entries().length - PICKER_ROWS;
    return Math.max(0, Math.min(state.artRow() - Math.floor(PICKER_ROWS / 2), last));
  };
  const visible = () => entries().slice(first(), first() + PICKER_ROWS);
  return (
    <Drawer state={state} title="Box art" size="wide">
      <For each={visible()}>
        {(name, index) => (
          <OptionRow
            state={state}
            active={first() + index() === state.artRow()}
            label={fitTitle(name, 240, fontSlot(state.font(), "bodyBold"))}
          />
        )}
      </For>
      <Spacer />
      <Note state={state}>
        {state.artNote() || (state.artFiles().length === 0 ? "No PNG files yet. Copy images to" : "")}
      </Note>
      <Note state={state}>
        {state.artNote() || state.artFiles().length > 0
          ? `${state.confirmGlyph()}: Use  ·  ${state.cancelGlyph()}: Back`
          : artFolder()}
      </Note>
    </Drawer>
  );
}
