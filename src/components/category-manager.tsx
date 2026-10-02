import { For, Show } from "solid-js";
import type { LauncherState } from "../state.ts";
import { fill } from "../i18n.ts";
import { hints, Prompt, withButton, type IconName, type Part } from "./icons.tsx";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

// Category rows visible at once. The rest of the drawer is for the note and
// two lines of hints, each of which can wrap to a second line in a language
// with longer words.
const ROWS = 4;

/** Create, rename, delete, reorder and hide categories. */
export function CategoryManagerOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const t = state.t;
  const entries = (): { label: string; value?: string; icon?: IconName }[] => [
    { label: t().newCategory, icon: "plus" },
    ...state.allCategories().map((item) => ({
      label: item.label,
      value: state.isHidden(item.id) ? t().hidden : "",
    })),
  ];
  // START hides the highlighted category, or shows it again when it is hidden.
  const toggleLabel = () => {
    const item = state.allCategories()[state.catRow() - 1];
    return item && state.isHidden(item.id) ? t().show : t().hide;
  };
  // While a row is armed for deletion, the note asks for the same button again.
  const armedNote = (): Part[] | undefined => {
    const armed = state.catArmed();
    return armed ? withButton(fill(t().deleteAgain, { name: state.categoryLabel(armed) }), "square") : undefined;
  };
  // Scroll so the highlighted row stays in view.
  const first = () => {
    const last = entries().length - ROWS;
    return Math.max(0, Math.min(state.catRow() - Math.floor(ROWS / 2), last));
  };
  const visible = () => entries().slice(first(), first() + ROWS);
  return (
    <Drawer state={state} title={t().categoriesRow} size="wide">
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
      <Show when={armedNote()} fallback={<Note state={state}>{state.catNote()}</Note>}>
        {(parts) => <Prompt state={state} parts={parts()} />}
      </Show>
      <Prompt
        state={state}
        parts={hints([state.confirmButton(), t().rename], ["triangle", t().create], ["square", t().remove])}
      />
      <Prompt
        state={state}
        parts={hints(["start", toggleLabel()], [["l", "r"], t().reorder], [state.cancelButton(), t().back])}
      />
    </Drawer>
  );
}
