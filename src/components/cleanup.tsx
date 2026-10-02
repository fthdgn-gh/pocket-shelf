import { fill } from "../i18n.ts";
import type { LauncherState } from "../state.ts";
import { hints, Prompt } from "./icons.tsx";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

/** Delete box art and backdrop files: the ones no title uses, or all of them. */
export function CleanupOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const t = state.t;
  const count = () => fill(t().cleanCount, { count: state.cleanCount() });
  // The result of the last press, the question before a delete, or what the mode does.
  const note = () => {
    if (state.cleanNote()) return state.cleanNote();
    if (state.cleanArmed()) return fill(t().cleanConfirm, { count: state.cleanCount() });
    return state.cleanEvery() ? t().cleanHelpAll : t().cleanHelpUnused;
  };
  return (
    <Drawer state={state} title={t().cleanArt} size="wide">
      <OptionRow
        state={state}
        active={state.cleanRow() === 0}
        label={t().cleanMode}
        value={state.cleanEvery() ? t().everything : t().unused}
        step="both"
      />
      <OptionRow state={state} active={state.cleanRow() === 1} label={t().remove} value={count()} step="right" />
      <Spacer />
      <Note state={state}>{note()}</Note>
      <Prompt
        state={state}
        parts={hints(
          [state.confirmButton(), state.cleanRow() === 0 ? t().change : t().remove],
          [state.cancelButton(), t().back],
        )}
      />
    </Drawer>
  );
}
