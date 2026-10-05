import { Match, Show, Switch } from "solid-js";
import { fill } from "../i18n.ts";
import type { LauncherState } from "../state.ts";
import { BUILD, buildLabel, releaseLabel } from "../updates.ts";
import { hints, Prompt } from "./icons.tsx";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";
import { ProgressBar } from "./progress.tsx";

const megabytes = (bytes: number) => (bytes / (1024 * 1024)).toFixed(1);

/** Checking for a newer build of the chosen channel, and downloading it. */
export function UpdateOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const update = state.update;
  const t = state.t;
  const step = update.step;
  const busy = () => step() === "checking" || step() === "downloading" || step() === "unpacking";
  return (
    <Drawer state={state} title={t().updates} size="wide">
      <OptionRow state={state} active={false} label={t().installedVersion} value={buildLabel(BUILD)} />
      <OptionRow
        state={state}
        active={false}
        label={t().updateChannel}
        value={t().updateChannels[state.updateChannel()]}
      />
      <Show when={update.release()}>
        {(release) => (
          <OptionRow
            state={state}
            active={step() === "available"}
            label={t().availableVersion}
            value={`${releaseLabel(release())}, ${megabytes(release().size)} MB`}
            step={step() === "available" ? "right" : undefined}
          />
        )}
      </Show>
      <Spacer />
      <Switch>
        <Match when={step() === "checking"}>
          <Note state={state}>{t().updateChecking}</Note>
        </Match>
        <Match when={step() === "current"}>
          <Note state={state}>{t().updateCurrent}</Note>
        </Match>
        <Match when={step() === "available"}>
          <Note state={state}>{t().updateAvailableHelp}</Note>
        </Match>
        <Match when={step() === "downloading" || step() === "unpacking"}>
          <ProgressBar theme={state.theme()} width={240} done={update.progress().done} total={update.progress().total} />
          <Note state={state}>
            {step() === "downloading"
              ? fill(t().updateDownloading, {
                  done: megabytes(update.progress().done),
                  total: megabytes(update.progress().total),
                })
              : fill(t().updateUnpacking, { done: update.progress().done, total: update.progress().total })}
          </Note>
        </Match>
        <Match when={step() === "ready" || step() === "failed"}>
          <Note state={state}>{update.status()}</Note>
        </Match>
      </Switch>
      <Prompt
        state={state}
        parts={
          busy()
            ? hints([state.cancelButton(), step() === "checking" ? t().close : t().stop])
            : step() === "available"
              ? hints([state.confirmButton(), t().updateDownload], [state.cancelButton(), t().updateLater])
              : step() === "ready"
                ? hints([state.confirmButton(), t().close])
                : hints([state.confirmButton(), t().checkAgain], [state.cancelButton(), t().close])
        }
      />
    </Drawer>
  );
}
