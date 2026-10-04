import { getOps } from "@pocketjs/framework/host";

/** What the status bar shows. A reading the host cannot give is null. */
export interface Status {
  hour: number;
  minute: number;
  /** The system's clock format; null when unknown (24-hour is then used). */
  clock24: boolean | null;
  /** Battery charge in percent. */
  battery: number | null;
  charging: boolean;
  /** Wi-Fi signal in percent, 0 when not connected. */
  wifi: number | null;
  /** Whether Bluetooth is switched on in Settings. */
  bluetooth: boolean | null;
}

/** Vita host extra (hosts/vita/src/status.rs). */
interface StatusHost {
  __status?(): string;
}

/**
 * Parse the host's `hour minute clock24 battery charging wifi bluetooth`
 * line. Each field is a number; -1 means the host could not read it.
 */
export function parseStatus(line: string): Status | null {
  const fields = line.trim().split(/\s+/).map(Number);
  if (fields.length !== 7 || fields.some((value) => !Number.isInteger(value))) return null;
  const [hour, minute, clock24, battery, charging, wifi, bluetooth] = fields as [
    number, number, number, number, number, number, number,
  ];
  if (hour < 0 || hour > 23 || minute < 0 || minute > 59) return null;
  return {
    hour,
    minute,
    clock24: clock24 < 0 ? null : clock24 === 1,
    battery: battery < 0 ? null : Math.min(battery, 100),
    charging: charging === 1,
    wifi: wifi < 0 ? null : Math.min(wifi, 100),
    bluetooth: bluetooth < 0 ? null : bluetooth === 1,
  };
}

/**
 * The current readings. A host without `__status` gives the time from
 * `Date` and nothing else.
 */
export function readStatus(): Status {
  const line = (getOps() as unknown as StatusHost).__status?.();
  const parsed = line === undefined ? null : parseStatus(line);
  if (parsed) return parsed;
  const now = new Date();
  return {
    hour: now.getHours(),
    minute: now.getMinutes(),
    clock24: null,
    battery: null,
    charging: false,
    wifi: null,
    bluetooth: null,
  };
}

/** Whether two readings would draw the same bar. */
export function sameStatus(a: Status | null, b: Status | null): boolean {
  if (a === null || b === null) return a === b;
  return (
    a.hour === b.hour &&
    a.minute === b.minute &&
    a.clock24 === b.clock24 &&
    a.battery === b.battery &&
    a.charging === b.charging &&
    a.wifi === b.wifi &&
    a.bluetooth === b.bluetooth
  );
}

/** "14:05", or "2:05 PM" on a 12-hour clock. An unknown format is written as 24-hour. */
export function formatClock(hour: number, minute: number, clock24: boolean | null): string {
  const mm = String(minute).padStart(2, "0");
  if (clock24 !== false) return `${String(hour).padStart(2, "0")}:${mm}`;
  const h12 = hour % 12 === 0 ? 12 : hour % 12;
  return `${h12}:${mm} ${hour < 12 ? "AM" : "PM"}`;
}

/**
 * Rings of the Wi-Fi symbol lit for a signal in percent: 0 (the dot alone) to
 * 3, like the Vita's four levels.
 */
export function wifiLevel(percent: number): number {
  return Math.max(0, Math.min(3, Math.floor(percent / 25)));
}
