// Startup timing, shown on the "Diagnostics" screen of the SELECT menu. The
// host records when each of its phases finished (`startup_mark` in
// hosts/vita/src/lib.rs); this module times the app's own steps.

interface TimingHost {
  __startupMarks?(): string;
  __scanReport?(): string;
  __processMs?(): number;
}
// The host's `ui` object. The framework's own accessor works only after
// mount, and this module runs before it.
const host = (): TimingHost => (globalThis as unknown as { ui?: TimingHost }).ui ?? {};

/** Milliseconds since the process started, on the host's clock when it has one. */
const now = (): number => host().__processMs?.() ?? Date.now();

const phases: [label: string, ms: number][] = [];
// When this module ran: close to the first statement of the bundle, after
// the framework's own modules.
const scriptStarted = now();

/** Run `work` and record how long it took under `label`. Only the first run per label counts. */
export function timed<T>(label: string, work: () => T): T {
  const started = now();
  try {
    return work();
  } finally {
    if (!phases.some(([known]) => known === label)) phases.push([label, now() - started]);
  }
}

let setupStarted = 0;
/** Call right before `mount`: the framework's own setup is timed up to `setupDone`. */
export function setupBegin(): void {
  setupStarted = now();
}
/** Call at the start of the root component. */
export function setupDone(): void {
  if (!phases.some(([known]) => known === "setup")) phases.push(["setup", now() - setupStarted]);
}

/** Call at the end of the entry file: how long the bundle's own code ran. */
export function scriptDone(): void {
  phases.push(["script", now() - scriptStarted]);
}

/** One line of the diagnostics screen. */
export interface DiagnosticEntry {
  label: string;
  value: string;
}

/**
 * The startup timing, in the order it happened. "Host" entries say when a
 * phase finished, in milliseconds since the process started; "App begin" is
 * on the same clock. The other "App" entries say how long a step took.
 */
export function startupEntries(): DiagnosticEntry[] {
  const entries: DiagnosticEntry[] = [];
  for (const mark of (host().__startupMarks?.() ?? "").split(",")) {
    const [label, ms] = mark.split("=");
    if (label && ms) entries.push({ label: `Host ${label}`, value: `at ${ms} ms` });
  }
  entries.push({ label: "App begin", value: `at ${scriptStarted} ms` });
  for (const [label, ms] of phases) entries.push({ label: `App ${label}`, value: `${ms} ms` });
  return [...entries, ...scanEntries()];
}

/**
 * What the last title scan found among PSM, PSP and PS1 titles (`psm.rs` and
 * `pspemu.rs` in the Vita host), as `Label=value` pairs:
 * `PSM folders=3,PSM source=app.db,PSP bubbles=2`. Nothing when this start
 * read the saved list instead of scanning.
 */
function scanEntries(): DiagnosticEntry[] {
  const report = host().__scanReport?.() ?? "";
  if (!report) return [];
  return report.split(",").flatMap((pair) => {
    const [key, value] = pair.split("=");
    return key && value ? [{ label: key, value }] : [];
  });
}
