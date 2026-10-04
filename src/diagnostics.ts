// Startup timing, shown on the "Diagnostics" screen of the SELECT menu. The
// host records when each of its phases finished (`startup_mark` in
// hosts/vita/src/lib.rs); this module times the app's own steps.

interface TimingHost {
  __startupMarks?(): string;
  __scanReport?(): string;
  __processMs?(): number;
  __bootLog?(): string;
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
  return [...entries, ...scanEntries(), ...bootEntries()];
}

/**
 * What the last title scan found among PSM, PSP and PS1 titles (`psm.rs` and
 * `pspemu.rs` in the Vita host), as `Label=value` pairs:
 * `PSM folders=3,PSM source=app.db,PSP titles=2`. Nothing when this start
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

/**
 * The boot plugin's last log lines (src/psp-boot/main.c), oldest first. A line
 * reads `age=3s path=ms0:/ISO/Game.cso boot iso`; it is shown as "Boot iso"
 * with the age and the file name (an EBOOT's folder, its title id).
 */
const DRIVERS: Record<number, string> = { 1: "Inferno", 2: "March33", 3: "NP9660", 4: "Inferno", 5: "ME" };

function bootEntries(): DiagnosticEntry[] {
  const log = host().__bootLog?.() ?? "";
  if (!log) return [];
  return log.split("\n").map((line) => {
    // Before an image boots: the boot config index of the ISO driver and the
    // file it boots. Each Adrenaline numbers its drivers its own way; these
    // are the numbers the plugin uses.
    // Before a boot: what the plugin waited for, the first module after the
    // XMB's main one, or the fallback wait with the XMB up.
    const trigger = /^trigger=(?:after )?(.+)$/.exec(line);
    if (trigger) {
      const name = trigger[1]!;
      return { label: "Boot trigger", value: name.length > 16 ? `${name.slice(0, 13)}...` : name };
    }
    const driver = /^driver=(\d+)( \(default\))? (\S+)$/.exec(line);
    if (driver) {
      const name = DRIVERS[Number(driver[1])] ?? driver[1]!;
      return { label: "Boot driver", value: `${name}${driver[2] ? " (default)" : ""} ${driver[3]}` };
    }
    const match = /^age=(\S+) path=(\S*(?: \S+)*?) (stale|bad kind|bad path|bad boot file|missing|boot \w+|refused)(?: code=(\S+))?$/.exec(line);
    if (!match) return { label: "Boot", value: line.slice(0, 32) };
    const [, age, path, outcome, code] = match;
    // An EBOOT is named by its folder, the title id.
    const parts = path!.split("/");
    const file = parts.at(-1) === "EBOOT.PBP" ? (parts.at(-2) ?? "") : (parts.at(-1) ?? "");
    const name = file.length > 16 ? `${file.slice(0, 13)}...` : file;
    return { label: `Boot ${outcome!.replace(/^boot /, "")}`, value: `${age} ${code ? `${code} ` : ""}${name}` };
  });
}
