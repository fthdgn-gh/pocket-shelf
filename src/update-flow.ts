import { batch, createSignal } from "solid-js";
import { writeFileSync } from "@pocketjs/framework/fs";
import { getOps } from "@pocketjs/framework/host";
import { fill, type Messages } from "./i18n.ts";
import { getText, netAvailable, saveFile } from "./net.ts";
import {
  BUILD,
  buildLabel,
  checkDue,
  loadUpdateRecord,
  parseReleases,
  pickUpdate,
  RELEASES_URL,
  releaseKey,
  releaseLabel,
  saveUpdateRecord,
  UPDATE_FILE,
  UPDATE_VERSION_FILE,
  type Release,
  type UpdateChannel,
} from "./updates.ts";

// hosts/vita/src/update.rs
interface UpdateHost {
  __updateUnpack?(): number;
  __updateInstall?(): number;
  __updateLaunch?(): number;
  __updateState?(): string;
  __updateResult?(): string;
}

const host = () => getOps() as unknown as UpdateHost;

/**
 * Where the drawer is:
 *  - "checking": asking GitHub for its releases;
 *  - "current": nothing newer on the channel;
 *  - "available": a release to download, with "Download" and "Later";
 *  - "downloading", "unpacking": getting it ready;
 *  - "ready": the package is unpacked and checked, with "Install";
 *  - "installing": installing the updater app, which then takes over;
 *  - "updated": the updater installed the release (shown after the restart);
 *  - "failed": a step went wrong; `status` says which.
 */
export type UpdateStep =
  | "checking"
  | "current"
  | "available"
  | "downloading"
  | "unpacking"
  | "ready"
  | "installing"
  | "updated"
  | "failed";

interface Deps {
  t: () => Messages;
  channel: () => UpdateChannel;
  /** Whether no other drawer is open, so something found at start can be shown. */
  idle: () => boolean;
}

/** Whether this build can download and install its own update. */
export const canUpdate = () => typeof host().__updateUnpack === "function" && netAvailable();

/** Checking GitHub for a newer build of the chosen channel, and installing it (SELECT menu -> Updates). */
export function createUpdateFlow(deps: Deps) {
  const t = deps.t;
  const [open, setOpen] = createSignal(false);
  const [step, setStep] = createSignal<UpdateStep>("checking");
  const [status, setStatus] = createSignal("");
  const [release, setRelease] = createSignal<Release | undefined>(undefined);
  const [progress, setProgress] = createSignal({ done: 0, total: 0 });

  let cancel: (() => void) | undefined;
  let turn = 0;
  // A host step is running: its state is read once per frame.
  let polling = false;
  // What to show once no other drawer is open: an update the check at start
  // found, or the updater's report after a restart.
  let pending: (() => void) | undefined;

  const fail = (message: string) =>
    batch(() => {
      setStep("failed");
      setStatus(message);
    });

  const stop = () => {
    turn++;
    cancel?.();
    cancel = undefined;
  };

  /** Ask GitHub; `found` gets the channel's update, or undefined when there is none. */
  const request = (found: (update: Release | undefined) => void, failed: (message: string) => void) => {
    const mine = ++turn;
    cancel = getText(RELEASES_URL, "", (result) => {
      if (mine !== turn) return;
      cancel = undefined;
      if (!result.ok) return failed(fill(t().updateUnreachable, { error: result.error }));
      const releases = result.status === 200 ? parseReleases(result.text) : null;
      if (!releases) return failed(fill(t().updateBadReply, { status: result.status }));
      saveUpdateRecord({ ...loadUpdateRecord(), checkedAt: Date.now() });
      found(pickUpdate(releases, deps.channel(), BUILD));
    });
  };

  const show = (update: Release | undefined) =>
    batch(() => {
      setRelease(update);
      setStep(update ? "available" : "current");
      setStatus("");
    });

  /** Open the drawer and check now (SELECT menu -> Check for updates). */
  const check = () => {
    if (polling) {
      setOpen(true);
      return;
    }
    stop();
    batch(() => {
      setOpen(true);
      setRelease(undefined);
      setStatus("");
      setStep("checking");
    });
    if (deps.channel() === "off") return fail(t().updateOff);
    if (!canUpdate()) return fail(t().updateNotSupported);
    request(show, fail);
  };

  /**
   * At start: the updater's report when it ran, else the check, at most once
   * a day and silent unless it finds an update not put off before.
   */
  const atStart = () => {
    const report = host().__updateResult?.() ?? "";
    if (report) {
      const [kind, ...rest] = report.split(" ");
      pending = () =>
        batch(() => {
          setRelease(undefined);
          if (kind === "ok") {
            setStep("updated");
            // The running build is the one the updater installed.
            setStatus(fill(t().updateDone, { version: buildLabel(BUILD) }));
          } else fail(fill(t().updateInstallFailed, { error: rest.join(" ") || "unknown" }));
        });
      return;
    }
    const record = loadUpdateRecord();
    if (!canUpdate() || !checkDue(deps.channel(), record, Date.now())) return;
    request(
      (update) => {
        if (update && releaseKey(update) !== record.later) pending = () => show(update);
      },
      () => {},
    );
  };

  const download = () => {
    const chosen = release();
    if (!chosen) return;
    const mine = ++turn;
    batch(() => {
      setStep("downloading");
      setProgress({ done: 0, total: chosen.size });
      setStatus("");
    });
    cancel = saveFile(
      chosen.url,
      UPDATE_FILE,
      (result) => {
        if (mine !== turn) return;
        cancel = undefined;
        if (!result.ok) return fail(fill(t().downloadFailed, { error: result.error }));
        if (result.status !== 200) return fail(fill(t().updateBadReply, { status: result.status }));
        runHost("unpacking", host().__updateUnpack);
      },
      (received, total) => {
        if (mine === turn) setProgress({ done: received, total: total || chosen.size });
      },
    );
  };

  /** Start a host step and follow its state in `frame`. */
  const runHost = (next: UpdateStep, start: (() => number) | undefined) => {
    if ((start?.() ?? -1) < 0) return fail(fill(t().updateFailed, { error: "busy" }));
    polling = true;
    batch(() => {
      setStep(next);
      setProgress({ done: 0, total: 0 });
      setStatus("");
    });
  };

  const confirm = () => {
    switch (step()) {
      case "available":
        download();
        break;
      case "ready": {
        // For the updater's screen; it leaves the version out without the file.
        const chosen = release();
        try {
          if (chosen) writeFileSync(UPDATE_VERSION_FILE, releaseLabel(chosen));
        } catch (error) {
          console.log(`Update version not saved: ${error}`);
        }
        runHost("installing", host().__updateInstall);
        break;
      }
      case "failed":
      case "current":
        check();
        break;
      case "updated":
        setOpen(false);
        break;
    }
  };

  /**
   * Back out: an offered update is put off, a download stops. A host step
   * (unpacking, installing) runs on; the drawer only closes.
   */
  const cancelStep = () => {
    const current = release();
    if (step() === "available" && current) saveUpdateRecord({ ...loadUpdateRecord(), later: releaseKey(current) });
    if (!polling) stop();
    setOpen(false);
  };

  /** Call once per frame. */
  const frame = () => {
    if (pending && !open() && deps.idle()) {
      const showPending = pending;
      pending = undefined;
      setOpen(true);
      showPending();
    }
    if (!polling) return;
    const [kind, ...rest] = (host().__updateState?.() ?? "error unavailable").split(" ");
    switch (kind) {
      case "busy":
        setProgress({ done: Number(rest[0]) || 0, total: Number(rest[1]) || 0 });
        return;
      case "idle":
      case "installing":
        return;
      case "done":
        polling = false;
        batch(() => {
          setStep("ready");
          setStatus(t().updateReady);
        });
        return;
      case "installed":
        polling = false;
        setOpen(true);
        setStatus(t().updateRestarting);
        // The process ends after this frame; the updater takes over.
        if ((host().__updateLaunch?.() ?? -1) < 0) fail(fill(t().updateInstallFailed, { error: "launch" }));
        return;
      default:
        polling = false;
        fail(fill(t().updateFailed, { error: rest.join(" ") || "unknown" }));
    }
  };

  return { open, step, status, release, progress, check, atStart, confirm, cancel: cancelStep, frame };
}

export type UpdateFlow = ReturnType<typeof createUpdateFlow>;
