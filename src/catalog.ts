import { getOps } from "@pocketjs/framework/host";
import { appTable } from "@pocketjs/framework/launcher";
import { categoryOf, type Platform } from "./categories.ts";
import type { Game } from "./types.ts";

/** How many tints titles cycle through (see TINT_ART and TINT_AMBIENT in components/art.tsx). */
export const TINT_COUNT = 6;

/** The tints as `0xRRGGBB`, for a title whose icon gives no color of its own. */
export const TINT_COLORS: readonly number[] = [0xf59e0b, 0x10b981, 0x3b82f6, 0x8b5cf6, 0xec4899, 0x06b6d4];

// The baked font has no trademark glyphs; they render as empty boxes. A mark
// between two words with no space ("PlayStation®Store") becomes a space.
export function cleanTitle(title: string): string {
  return title
    .replace(/(\w)[®™©℠](?=\w)/g, "$1 ")
    .replace(/[®™©℠]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

/**
 * Cuts a title to fit `maxWidth` pixels in the given font slot, measured with
 * the baked font, so wide faces (monospace) and narrow ones both fit.
 */
export function fitTitle(title: string, maxWidth: number, slot: number): string {
  const ops = getOps();
  const width = (text: string) => ops.measureText(text, slot);
  if (width(title) <= maxWidth) return title;
  let end = title.length;
  while (end > 1 && width(`${title.slice(0, end).trimEnd()}...`) > maxWidth) end--;
  return `${title.slice(0, end).trimEnd()}...`;
}

/**
 * `text` broken into lines of at most `maxWidth` pixels in the given font
 * slot. Breaks fall between words; a word wider than a line stays whole.
 */
export function wrapText(text: string, maxWidth: number, slot: number): string[] {
  const ops = getOps();
  const lines: string[] = [];
  let line = "";
  for (const word of text.split(" ")) {
    const next = line ? `${line} ${word}` : word;
    if (line && ops.measureText(next, slot) > maxWidth) {
      lines.push(line);
      line = word;
    } else line = next;
  }
  lines.push(line);
  return lines;
}

/** The end of `text` that fits `maxWidth` pixels, led by "..." when it was cut. */
export function fitTail(text: string, maxWidth: number, slot: number): string {
  const ops = getOps();
  const width = (value: string) => ops.measureText(value, slot);
  if (width(text) <= maxWidth) return text;
  let start = 1;
  while (start < text.length - 1 && width(`...${text.slice(start)}`) > maxWidth) start++;
  return `...${text.slice(start)}`;
}

export interface Catalog {
  games: Game[];
  /** True when the host reports installed titles that `appLaunch` can start. */
  native: boolean;
}

// Adrenaline, the PSP emulator, is installed as PSPEMUCFW. The bubbles it
// makes for single PSP and PS1 games use the same prefix with a number, for
// example PSPEMU001.
const ADRENALINE_ID = "PSPEMUCFW";
const ADRENALINE_BUBBLE_PREFIX = "PSPEMU";

// The host reports every application in the firmware's `vs0:app`, ids `NPXS`
// plus digits. Most are background services and dialogs. These are the ones
// with a bubble on the home screen, the same set vita-launcher lists, less
// its Package Installer.
const SYSTEM_APP_PREFIX = "NPXS";
const SYSTEM_APPS: ReadonlySet<string> = new Set([
  "NPXS10000", // near
  "NPXS10001", // Party
  "NPXS10002", // PlayStation Store
  "NPXS10003", // Internet Browser
  "NPXS10004", // Photos
  "NPXS10006", // Friends
  "NPXS10008", // Trophy Collection
  "NPXS10009", // Music
  "NPXS10010", // Videos
  "NPXS10014", // Messages
  "NPXS10015", // Settings
  "NPXS10026", // Content Manager
  "NPXS10072", // Email
  "NPXS10091", // Calendar
  "NPXS10094", // Parental Controls
  "NPXS10098", // PS4 Link
]);

/**
 * Whether an installed title belongs in the launcher. Adrenaline's game
 * bubbles are left out: each one is a shortcut into Adrenaline, not a title
 * of its own. Adrenaline itself stays. Of the system applications, the ones
 * in `SYSTEM_APPS` are listed.
 */
export function isListed(titleId: string): boolean {
  if (titleId.startsWith(SYSTEM_APP_PREFIX)) return SYSTEM_APPS.has(titleId);
  return titleId === ADRENALINE_ID || !titleId.startsWith(ADRENALINE_BUBBLE_PREFIX);
}

/**
 * Ask the host to read the installed titles again. The Vita host keeps the
 * list in a file between starts (hosts/vita/src/installed.rs) and scans only
 * when asked; this takes seconds. Hosts without the extra do nothing.
 */
export function rescanTitles(): void {
  (getOps() as unknown as { __appRescan?(): number }).__appRescan?.();
}

const GENRES: Record<Platform, string> = { vita: "PS Vita", psm: "PS Mobile", psp: "PSP", ps1: "PS1" };

/** The platform an entry names; Vita for an entry with none or an unknown one. */
function platformOf(value: unknown): Platform {
  return value === "psm" || value === "psp" || value === "ps1" ? value : "vita";
}

/**
 * The host's installed-title table (kind "native"), less the titles
 * `isListed` leaves out. Hosts without app navigation report no table, which
 * leaves the list empty.
 */
export function loadCatalog(): Catalog {
  const table = appTable();
  if (table?.kind !== "native") return { native: false, games: [] };
  const apps = table.apps.filter((app) => isListed(app.id));
  const games: Game[] = apps.map((app, index) => {
    // The Vita host adds the platform to each entry (hosts/vita/src/installed.rs).
    const platform = platformOf((app as { platform?: unknown }).platform);
    return {
      title: cleanTitle(app.title),
      id: app.id,
      genre: GENRES[platform],
      category: categoryOf(app.id, platform),
      tint: index % TINT_COUNT,
    };
  });
  return { native: true, games };
}
