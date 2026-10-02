import { fill } from "../i18n.ts";
import type { LauncherState } from "../state.ts";
import { hints, Prompt } from "./icons.tsx";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

/** Read the installed titles again, after one was installed or removed. */
export function RescanOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const t = state.t;
  return (
    <Drawer state={state} title={t().rescan} size="wide">
      <OptionRow
        state={state}
        active
        label={t().scan}
        value={fill(t().scrapeCountAll, { count: state.titleCount() })}
        step="right"
      />
      <Spacer />
      <Note state={state}>{state.rescanNote() || t().scanHelp}</Note>
      <Prompt state={state} parts={hints([state.confirmButton(), t().scan], [state.cancelButton(), t().back])} />
    </Drawer>
  );
}
