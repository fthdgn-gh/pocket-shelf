import { batch, createSignal } from "solid-js";
import { getOps } from "@pocketjs/framework/host";
import { fill, type Messages } from "./i18n.ts";
import { getText, netAvailable, saveFile } from "./net.ts";
import {
  BUILD,
  checkDue,
  loadUpdateRecord,
  parseReleases,
  pickUpdate,
  RELEASES_URL,
  releaseKey,
  saveUpdateRecord,
  UPDATE_FILE,
  type Release,
  type UpdateChannel,
} from "./updates.ts";

interface UpdateHost {
  __updateUnpack?(): number;
  __updateState?(): string;
}

const host = () => getOps() as unknown as UpdateHost;

/**
 * Where the drawer is:
 *  - "checking": asking GitHub for its releases;
 *  - "current": nothing newer on the channel;
 *  - "available": a release to download, with "Download" and "Later";
 *  - "downloading", "unpacking": getting it ready;
 *  - "ready": the package is unpacked and checked;
 *  - "failed": a step went wrong; `status` says which.
 */
export type UpdateStep = "checking" | "current" | "available" | "downloading" | "unpacking" | "ready" | "failed";

interface Deps {
  t: () => Messages;
  channel: () => UpdateChannel;
  /** Whether no other drawer is open, so an update found at start can be shown. */
  idle: () => boolean;
}

/** Whether this build can download and unpack its own update. */
export const canUpdate = () => typeof host().__updateUnpack === "function" && netAvailable();

/** Checking GitHub for a newer build of the chosen channel, and getting it (SELECT menu -> Updates). */
export function createUpdateFlow(deps: Deps) {
  const t = deps.t;
  const [open, setOpen] = createSignal(false);
  const [step, setStep] = createSignal<UpdateStep>("checking");
  const [status, setStatus] = createSignal("");
  const [release, setRelease] = createSignal<Release | undefined>(undefined);
  const [progress, setProgress] = createSignal({ done: 0, total: 0 });

  let cancel: (() => void) | undefined;
  let turn = 0;
  let unpacking = false;
  // An update found by the check at start, shown once no other drawer is open.
  let offer: Release | undefined;

  const fail = (message: string) =>
    batch(() => {
      setStep("failed");
      setStatus(message);
    });

  const stop = () => {
    turn++;
    cancel?.();
    cancel = undefined;
    unpacking = false;
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

  /** The check at start: at most once a day, and silent unless it finds an update not put off before. */
  const checkAtStart = () => {
    const record = loadUpdateRecord();
    if (!canUpdate() || !checkDue(deps.channel(), record, Date.now())) return;
    request(
      (update) => {
        if (update && releaseKey(update) !== record.later) offer = update;
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
        unpack();
      },
      (received, total) => {
        if (mine === turn) setProgress({ done: received, total: total || chosen.size });
      },
    );
  };

  const unpack = () => {
    if ((host().__updateUnpack?.() ?? -1) < 0) return fail(fill(t().updateFailed, { error: "busy" }));
    unpacking = true;
    batch(() => {
      setStep("unpacking");
      setProgress({ done: 0, total: 0 });
    });
  };

  const confirm = () => {
    switch (step()) {
      case "available":
        download();
        break;
      case "failed":
      case "current":
        check();
        break;
      case "ready":
        setOpen(false);
        break;
    }
  };

  /** Back out: a running step stops, an offered update is put off, else the drawer closes. */
  const cancelStep = () => {
    const current = release();
    if (step() === "available" && current) saveUpdateRecord({ ...loadUpdateRecord(), later: releaseKey(current) });
    stop();
    setOpen(false);
  };

  /** Call once per frame. */
  const frame = () => {
    if (offer && !open() && deps.idle()) {
      const update = offer;
      offer = undefined;
      setOpen(true);
      show(update);
    }
    if (!unpacking) return;
    const [kind, ...rest] = (host().__updateState?.() ?? "error unavailable").split(" ");
    if (kind === "busy") {
      setProgress({ done: Number(rest[0]) || 0, total: Number(rest[1]) || 0 });
      return;
    }
    if (kind === "idle") return;
    unpacking = false;
    if (kind === "done") {
      batch(() => {
        setStep("ready");
        setStatus(t().updateReady);
      });
    } else fail(fill(t().updateFailed, { error: rest.join(" ") || "unknown" }));
  };

  return { open, step, status, release, progress, check, checkAtStart, confirm, cancel: cancelStep, frame };
}

export type UpdateFlow = ReturnType<typeof createUpdateFlow>;
