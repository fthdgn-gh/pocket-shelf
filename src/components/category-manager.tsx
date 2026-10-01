import { For } from "solid-js";
import { PICKER_ROWS, type LauncherState } from "../state.ts";
import { hints, Prompt, type IconName, type Part } from "./icons.tsx";
import { Drawer, OptionRow, Spacer } from "./panel.tsx";

/** Create, rename, delete, reorder and hide categories. */
export function CategoryManagerOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const entries = (): { label: string; value?: string; icon?: IconName }[] => [
    { label: "New category", icon: "plus" },
    ...state.allCategories().map((item) => ({
      label: item.label,
      value: state.isHidden(item.id) ? "Hidden" : "",
    })),
  ];
  // START hides the highlighted category, or shows it again when it is hidden.
  const toggleLabel = () => {
    const item = state.allCategories()[state.catRow() - 1];
    return item && state.isHidden(item.id) ? "Show" : "Hide";
  };
  // While a row is armed for deletion, the note starts with the button to press again.
  const note = (): Part[] => (state.catArmed() ? ["Press", { icon: "square" }, state.catNote()] : [state.catNote() || " "]);
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
            icon={entry.icon}
          />
        )}
      </For>
      <Spacer />
      <Prompt state={state} parts={note()} />
      <Prompt
        state={state}
        parts={hints([state.confirmButton(), "Rename"], ["triangle", "New"], ["square", "Delete"])}
      />
      <Prompt
        state={state}
        parts={hints(["start", toggleLabel()], [["l", "r"], "Move"], [state.cancelButton(), "Back"])}
      />
    </Drawer>
  );
}
