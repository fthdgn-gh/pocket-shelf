import { For } from "solid-js";
import { startupEntries } from "../diagnostics.ts";
import type { LauncherState } from "../state.ts";
import { hints, Prompt } from "./icons.tsx";
import { Drawer, OptionRow, Spacer } from "./panel.tsx";

/** Rows visible at once; the list scrolls with the highlight. */
const ROWS = 7;

/** Startup timing of this launch: when each host phase finished and how long each app step took. */
export function DiagnosticsOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const t = state.t;
  const entries = startupEntries();
  const row = () => state.diagnosticsRow() % entries.length;
  const first = () => Math.max(0, Math.min(row() - Math.floor(ROWS / 2), entries.length - ROWS));
  const visible = () => entries.slice(first(), first() + ROWS);
  return (
    <Drawer state={state} title={t().diagnostics} size="wide">
      <For each={visible()}>
        {(entry, index) => (
          <OptionRow state={state} active={first() + index() === row()} label={entry.label} value={entry.value} />
        )}
      </For>
      <Spacer />
      <Prompt state={state} parts={hints(["dpadVertical", t().navigate], [state.cancelButton(), t().back])} />
    </Drawer>
  );
}
