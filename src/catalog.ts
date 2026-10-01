import { getOps } from "@pocketjs/framework/host";
import { appTable } from "@pocketjs/framework/launcher";
import { categoryOf } from "./categories.ts";
import type { CategoryId, Game } from "./types.ts";

// Gradients behind each title's art. Class strings are compiled at build time,
// so each one is a full literal and titles cycle through the list.
const GRADIENTS = [
  "absolute inset-0 rounded-lg bg-gradient-to-b from-amber-500 to-red-600",
  "absolute inset-0 rounded-lg bg-gradient-to-b from-emerald-500 to-teal-700",
  "absolute inset-0 rounded-lg bg-gradient-to-b from-blue-500 to-indigo-700",
  "absolute inset-0 rounded-lg bg-gradient-to-b from-purple-500 to-violet-800",
  "absolute inset-0 rounded-lg bg-gradient-to-b from-pink-500 to-rose-600",
  "absolute inset-0 rounded-lg bg-gradient-to-b from-cyan-400 to-blue-600",
];

// The baked font has no trademark glyphs; they render as empty boxes.
export function cleanTitle(title: string): string {
  return title.replace(/[®™©℠]/g, "").replace(/\s+/g, " ").trim();
}

/** Cards and rows do not wrap, so long names are cut to fit their width. */
export function shortTitle(title: string, max: number): string {
  return title.length > max ? `${title.slice(0, max - 3).trimEnd()}...` : title;
}

/**
 * Adds placeholder titles after the real ones so the grid and list scrolling
 * can be tried with few installed apps. Set to false to hide them.
 */
const SHOW_TEST_TITLES = true;

const TEST_TITLES: [title: string, genre: string, category: CategoryId][] = [
  ["Neon Drift", "Racing", "games"],
  ["Castle Siege", "Strategy", "games"],
  ["Star Harbor", "Adventure", "games"],
  ["Pixel Quest", "RPG", "games"],
  ["Sky Runner", "Platformer", "games"],
  ["Deep Blue", "Simulation", "games"],
  ["Iron Fist Arena", "Fighting", "games"],
  ["Moon Garden", "Puzzle", "games"],
  ["Turbo Kart Rally", "Racing", "games"],
  ["Shadow Temple", "Action", "games"],
  ["Media Player", "Video", "apps"],
  ["Web Browser", "Internet", "apps"],
  ["Music Box", "Audio", "apps"],
  ["Photo Viewer", "Images", "apps"],
  ["File Manager", "Tools", "homebrew"],
  ["Retro Menu", "Emulation", "homebrew"],
  ["Save Editor", "Tools", "homebrew"],
  ["Plugin Loader", "System", "homebrew"],
  ["Tiny Synth", "Audio", "homebrew"],
];

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

/**
 * The host's installed-title table (kind "native"). Hosts without app
 * navigation report no table, which leaves the list empty.
 */
export function loadCatalog(): Catalog {
  const table = appTable();
  if (table?.kind !== "native") return { native: false, games: [] };
  const games: Game[] = table.apps.map((app, index) => ({
    title: cleanTitle(app.title),
    id: app.id,
    genre: "PS Vita",
    category: categoryOf(app.id),
    gradient: GRADIENTS[index % GRADIENTS.length],
  }));
  if (SHOW_TEST_TITLES) {
    TEST_TITLES.forEach(([title, genre, category], index) => {
      games.push({
        title,
        // Not a real title id: the host refuses to launch it and has no icon for it.
        id: `TEST${String(index).padStart(5, "0")}`,
        genre,
        category,
        gradient: GRADIENTS[(games.length) % GRADIENTS.length],
      });
    });
  }
  return { native: true, games };
}
