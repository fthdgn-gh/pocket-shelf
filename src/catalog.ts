import { getOps } from "@pocketjs/framework/host";
import { appTable } from "@pocketjs/framework/launcher";
import { categoryOf } from "./categories.ts";
import type { Game } from "./types.ts";

/** How many tints titles cycle through (see TINT_ART and TINT_AMBIENT in components/art.tsx). */
export const TINT_COUNT = 6;

/** The tints as `0xRRGGBB`, for a title whose icon gives no color of its own. */
export const TINT_COLORS: readonly number[] = [0xf59e0b, 0x10b981, 0x3b82f6, 0x8b5cf6, 0xec4899, 0x06b6d4];

// The baked font has no trademark glyphs; they render as empty boxes.
export function cleanTitle(title: string): string {
  return title.replace(/[®™©℠]/g, "").replace(/\s+/g, " ").trim();
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
    tint: index % TINT_COUNT,
  }));
  return { native: true, games };
}
