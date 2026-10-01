import { For } from "solid-js";
import { glyph } from "@pocketjs/framework/modality";
import { PICKER_ROWS, type LauncherState } from "../state.ts";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

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
  return (
    <Drawer state={state} title="Categories" size="wide">
      <For each={visible()}>
        {(entry, index) => (
          <OptionRow
            state={state}
            active={first() + index() === state.catRow()}
            label={entry.label}
            value={entry.value}
          />
        )}
      </For>
      <Spacer />
      <Note state={state}>{state.catNote() || " "}</Note>
      <Note state={state}>
        {`${state.confirmGlyph()}: Rename  ${glyph("triangle")}: New  ${glyph("square")}: Delete`}
      </Note>
      <Note state={state}>{`START: Hide / Show  L R: Move  ${state.cancelGlyph()}: Back`}</Note>
    </Drawer>
  );
}
