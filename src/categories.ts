import { readFileSync, writeFileSync } from "@pocketjs/framework/fs";
import type { CategoryId } from "./types.ts";

/** A category whose label comes from the language's `categories` texts. */
export type CategoryName = "recent" | "favorites" | "search" | "games" | "system" | "homebrew" | "psm" | "psp" | "psx";

export interface Category {
  id: CategoryId;
  /** The label the user typed, or the English label of a category with a `name`. */
  label: string;
  /** Set for every category the user did not create: the key of its translated label. */
  name?: CategoryName;
  /** True for categories the user created. */
  custom?: boolean;
  /** True for a category the launcher fills itself; a title cannot be moved into it. */
  smart?: boolean;
}

// The ids carry a prefix so they cannot match one made from a label the user
// typed before these categories existed.
export const RECENT_ID = "smart-recent";
export const FAVORITES_ID = "smart-favorites";

/**
 * Categories filled by the launcher: the titles started last, newest first,
 * and the titles marked as favorite in the editor. They sit before the other
 * tabs until the user reorders them, and are left out while they are empty.
 */
/**
 * The tab of search results. It is not part of `SMART_CATEGORIES`: it exists
 * only while a search is open, always first, and the category manager does
 * not list it.
 */
export const SEARCH_ID = "smart-search";
export const SEARCH_CATEGORY: Category = { id: SEARCH_ID, label: "Search", name: "search", smart: true };

export const SMART_CATEGORIES: readonly Category[] = [
  { id: RECENT_ID, label: "Last Played", name: "recent", smart: true },
  { id: FAVORITES_ID, label: "Favorites", name: "favorites", smart: true },
];

/** The firmware's own applications. */
export const SYSTEM_ID = "system";
/**
 * PlayStation Mobile titles. Added after the others existed, so its id has a
 * prefix no label the user typed turns into.
 */
export const PSM_ID = "builtin-psm";
/** PSP games and PS1 (PSX) games, with the same kind of id as `PSM_ID`. */
export const PSP_ID = "builtin-psp";
export const PSX_ID = "builtin-psx";

/** Where a title lives, as the host reports it: the Vita's own titles, PSM, PSP or PSX (PS1 games). */
export type Platform = "vita" | "psm" | "psp" | "psx";

/** The default tab order: the system applications last. */
export const BUILTIN_CATEGORIES: readonly Category[] = [
  { id: "games", label: "Games", name: "games" },
  { id: "homebrew", label: "Homebrew", name: "homebrew" },
  { id: PSM_ID, label: "PS Mobile", name: "psm" },
  { id: PSP_ID, label: "PSP", name: "psp" },
  { id: PSX_ID, label: "PSX", name: "psx" },
  { id: SYSTEM_ID, label: "System", name: "system" },
];

/**
 * A built-in category with no titles is left out of the tab bar. Set to true to
 * show all of them always. Categories the user created always show.
 */
export const SHOW_EMPTY_CATEGORIES = false;

export const CATEGORY_LABEL_MAX = 16;
export const CATEGORY_ID_PATTERN = /^[a-z0-9-]{1,32}$/;

/**
 * Category of an installed title. PSP and PS1 (PSX) games are known by the
 * platform the host reports; the rest by title id: retail games use PCS ids,
 * system applications NPXS ids, PlayStation Mobile titles NPNA, NPOA, NPPA
 * or NPQA and five digits, and everything else is homebrew.
 */
export function categoryOf(titleId: string, platform?: Platform): CategoryId {
  if (platform === "psp") return PSP_ID;
  if (platform === "psx") return PSX_ID;
  if (titleId.startsWith("PCS")) return "games";
  if (titleId.startsWith("NPXS")) return SYSTEM_ID;
  if (isPsmId(titleId)) return PSM_ID;
  return "homebrew";
}

/** A PlayStation Mobile title id (`NPNA00001`), as `psm.rs` lists them. */
export function isPsmId(titleId: string): boolean {
  return /^NP[NOPQ]A\d{5}$/.test(titleId);
}

export function cycleCategory(list: readonly CategoryId[], current: CategoryId, delta: number): CategoryId {
  if (list.length === 0) return current;
  const index = list.indexOf(current);
  return list[((index < 0 ? 0 : index) + delta + list.length) % list.length];
}

/** A category id from its label, unique among `taken`. */
export function makeCategoryId(label: string, taken: ReadonlySet<string>): string {
  const slug =
    label
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 24) || "category";
  let id = slug;
  for (let n = 2; taken.has(id); n++) id = `${slug}-${n}`;
  return id;
}

// ux0:/data/PocketShelf/categories.json
//   custom  the categories the user created: [{ "id": "emulators", "label": "Emulators" }]
//   order   every category id, in tab order
//   hidden  ids left out of the tab bar
const CATEGORIES_FILE = "categories.json";

export interface CategoryConfig {
  custom: Category[];
  order: string[];
  hidden: string[];
}

const ids = (value: unknown): string[] =>
  Array.isArray(value)
    ? [...new Set(value.filter((id): id is string => typeof id === "string" && CATEGORY_ID_PATTERN.test(id)))]
    : [];

export function loadCategoryConfig(): CategoryConfig {
  const config: CategoryConfig = { custom: [], order: [], hidden: [] };
  try {
    const object = JSON.parse(readFileSync(CATEGORIES_FILE, "utf8")) as {
      custom?: Partial<Category>[];
      order?: unknown;
      hidden?: unknown;
    };
    const seen = new Set([SEARCH_CATEGORY, ...SMART_CATEGORIES, ...BUILTIN_CATEGORIES].map((item) => item.id));
    for (const value of Array.isArray(object.custom) ? object.custom : []) {
      const id = value?.id;
      const label = typeof value?.label === "string" ? value.label.trim().slice(0, CATEGORY_LABEL_MAX) : "";
      if (typeof id !== "string" || !CATEGORY_ID_PATTERN.test(id) || seen.has(id) || !label) continue;
      seen.add(id);
      config.custom.push({ id, label, custom: true });
    }
    config.order = ids(object.order);
    config.hidden = ids(object.hidden);
  } catch {
    // No saved categories, or the host has no fs namespace.
  }
  return config;
}

export function saveCategoryConfig(config: CategoryConfig): void {
  try {
    const plain = {
      custom: config.custom.map(({ id, label }) => ({ id, label })),
      order: config.order,
      hidden: config.hidden,
    };
    writeFileSync(CATEGORIES_FILE, `${JSON.stringify(plain, null, 2)}\n`);
  } catch (error) {
    console.log(`Categories not saved: ${error}`);
  }
}
