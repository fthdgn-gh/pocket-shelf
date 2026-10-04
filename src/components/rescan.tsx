import { Show } from "solid-js";
import { fill } from "../i18n.ts";
import type { LauncherState } from "../state.ts";
import { hints, Prompt } from "./icons.tsx";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";
import { ProgressBar } from "./progress.tsx";

/** Read the installed titles again, after one was installed or removed. */
export function RescanOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const t = state.t;
  const running = () => {
    const now = state.rescanProgress();
    return now?.state === "running" ? now : undefined;
  };
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
      <Show when={running()} fallback={<Note state={state}>{state.rescanNote() || t().scanHelp}</Note>}>
        {(now) => (
          <>
            <ProgressBar theme={state.theme()} width={240} done={now().done} total={now().total} />
            <Note state={state}>
              {fill(t().scanStep, { phase: t().scanPhases[now().phase], done: now().done, total: now().total })}
            </Note>
          </>
        )}
      </Show>
      <Show when={!running()}>
        <Prompt state={state} parts={hints([state.confirmButton(), t().scan], [state.cancelButton(), t().back])} />
      </Show>
    </Drawer>
  );
}
