import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "@pocketjs/framework/fs";
import { CATEGORY_ID_PATTERN } from "./categories.ts";
import type { CategoryId } from "./types.ts";

/** What the user changed about one title. Missing fields use the defaults. */
export interface TitleOverride {
  category?: CategoryId;
  title?: string;
  /** File name in the art folder, or "icon" for the title's own icon. */
  art?: string;
  /** File name in the backdrops folder, or "none" to draw no backdrop. */
  backdrop?: string;
  /** Present when the title is in the "Favorites" category. */
  favorite?: true;
}

export type Overrides = Record<string, TitleOverride>;

export const TITLE_MAX = 40;

/** `art` value meaning "show the title's own icon, even if a file matches its name". */
export const USE_ICON = "icon";

/** `backdrop` value meaning "draw no picture behind this title". */
export const NO_BACKDROP = "none";

// One JSON file per title in ux0:/data/PocketShelf/titles/<title id>.json,
// so a title's changes can be copied, deleted or edited by hand.
const TITLES_DIR = "titles";
const ID_PATTERN = /^[A-Za-z0-9_-]{1,32}$/;

// Text last read or written per title id; unchanged titles are not rewritten.
const onDisk = new Map<string, string>();

const fileOf = (id: string) => `${TITLES_DIR}/${id}.json`;
const render = (change: TitleOverride) => `${JSON.stringify(change, null, 2)}\n`;

const pngName = (name: string) => /\.png$/i.test(name) && name.length <= 96;

/** Keep only valid fields, so a hand-edited file cannot break the launcher. */
function sanitize(value: unknown): TitleOverride {
  const raw = (value && typeof value === "object" ? value : {}) as TitleOverride;
  const clean: TitleOverride = {};
  if (typeof raw.category === "string" && CATEGORY_ID_PATTERN.test(raw.category)) clean.category = raw.category;
  if (typeof raw.title === "string" && raw.title.trim()) clean.title = raw.title.trim().slice(0, TITLE_MAX);
  if (raw.art === USE_ICON || (typeof raw.art === "string" && pngName(raw.art))) {
    clean.art = raw.art;
  }
  if (raw.backdrop === NO_BACKDROP || (typeof raw.backdrop === "string" && pngName(raw.backdrop))) {
    clean.backdrop = raw.backdrop;
  }
  if (raw.favorite === true) clean.favorite = true;
  return clean;
}

/** The art or backdrop files the titles' changes name. */
export function filesInUse(overrides: Overrides, kind: "art" | "backdrop"): Set<string> {
  const used = new Set<string>();
  for (const change of Object.values(overrides)) {
    const file = change[kind];
    if (file !== undefined && file !== USE_ICON && file !== NO_BACKDROP) used.add(file);
  }
  return used;
}

/**
 * `overrides` without the art and backdrop files they name. The choices that
 * name no file ("icon", "none") stay.
 */
export function withoutFiles(overrides: Overrides): Overrides {
  const next: Overrides = {};
  for (const [id, change] of Object.entries(overrides)) {
    const kept: TitleOverride = { ...change };
    if (kept.art !== undefined && kept.art !== USE_ICON) delete kept.art;
    if (kept.backdrop !== undefined && kept.backdrop !== NO_BACKDROP) delete kept.backdrop;
    if (Object.keys(kept).length > 0) next[id] = kept;
  }
  return next;
}

// Every title's changes in one file, ux0:/data/PocketShelf/titles.cache.json.
// Reading one file per title cost 0.43 s of each start with a hundred
// titles. The per-title files stay the source: this file is rewritten with
// them, and `reloadOverrides` rebuilds it from them.
const CACHE_FILE = "titles.cache.json";
const CACHE_VERSION = 1;

function readCache(): Overrides | null {
  try {
    const raw = JSON.parse(readFileSync(CACHE_FILE, "utf8")) as { version?: unknown; titles?: unknown };
    if (raw.version !== CACHE_VERSION || !raw.titles || typeof raw.titles !== "object") return null;
    const result: Overrides = {};
    for (const [id, value] of Object.entries(raw.titles)) {
      if (!ID_PATTERN.test(id)) continue;
      const clean = sanitize(value);
      if (Object.keys(clean).length === 0) continue;
      result[id] = clean;
      onDisk.set(id, render(clean));
    }
    return result;
  } catch {
    return null;
  }
}

function writeCache(overrides: Overrides): void {
  writeFileSync(CACHE_FILE, `${JSON.stringify({ version: CACHE_VERSION, titles: overrides })}\n`);
}

/**
 * The titles' changes: from the one-file cache, or from the per-title files
 * when there is no cache yet. A per-title file edited, added or deleted by
 * hand is picked up by `reloadOverrides`.
 */
export function loadOverrides(): Overrides {
  return readCache() ?? reloadOverrides();
}

/** Read the per-title files again and rebuild the cache from them. */
export function reloadOverrides(): Overrides {
  const result = readFolder();
  try {
    writeCache(result);
  } catch {
    // No fs namespace on this host, or the data folder is not writable.
  }
  return result;
}

function readFolder(): Overrides {
  const result: Overrides = {};
  onDisk.clear();
  try {
    mkdirSync(TITLES_DIR);
    for (const name of readdirSync(TITLES_DIR)) {
      if (!name.endsWith(".json")) continue;
      const id = name.slice(0, -".json".length);
      if (!ID_PATTERN.test(id)) continue;
      try {
        const clean = sanitize(JSON.parse(readFileSync(fileOf(id), "utf8")));
        if (Object.keys(clean).length === 0) continue;
        result[id] = clean;
        onDisk.set(id, render(clean));
      } catch {
        // An unreadable or malformed file is skipped and left as it is.
      }
    }
  } catch {
    // No fs namespace on this host.
  }
  return result;
}

/** Write the titles whose changes differ from disk and delete files for titles that were reset. */
export function saveOverrides(overrides: Overrides): void {
  try {
    let changed = false;
    for (const [id, change] of Object.entries(overrides)) {
      if (!ID_PATTERN.test(id)) continue;
      const text = render(change);
      if (onDisk.get(id) === text) continue;
      writeFileSync(fileOf(id), text);
      onDisk.set(id, text);
      changed = true;
    }
    for (const id of [...onDisk.keys()]) {
      if (id in overrides) continue;
      rmSync(fileOf(id), { force: true });
      onDisk.delete(id);
      changed = true;
    }
    if (changed) writeCache(overrides);
  } catch (error) {
    console.log(`Title changes not saved: ${error}`);
  }
}
