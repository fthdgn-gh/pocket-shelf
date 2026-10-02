import { readFileSync, writeFileSync } from "@pocketjs/framework/fs";

/** How many titles the "Last Played" category keeps. */
export const RECENT_MAX = 15;

/** `list` with `id` moved to the front, cut to `max` ids. */
export function pushRecent(list: readonly string[], id: string, max = RECENT_MAX): string[] {
  return [id, ...list.filter((other) => other !== id)].slice(0, max);
}

// ux0:/data/PocketShelf/recent.json: title ids, the one started last first.
const RECENT_FILE = "recent.json";
const TITLE_ID_PATTERN = /^[A-Za-z0-9]{9}$/;

export function loadRecent(): string[] {
  try {
    const raw = JSON.parse(readFileSync(RECENT_FILE, "utf8")) as unknown;
    if (!Array.isArray(raw)) return [];
    const ids = raw.filter((id): id is string => typeof id === "string" && TITLE_ID_PATTERN.test(id));
    return [...new Set(ids)].slice(0, RECENT_MAX);
  } catch {
    // Nothing was started yet, or the host has no fs namespace.
    return [];
  }
}

export function saveRecent(list: readonly string[]): void {
  try {
    writeFileSync(RECENT_FILE, `${JSON.stringify(list)}\n`);
  } catch (error) {
    console.log(`Last played not saved: ${error}`);
  }
}
