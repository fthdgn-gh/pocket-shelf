import { readFileSync, writeFileSync } from "@pocketjs/framework/fs";
import type { CategoryId } from "./types.ts";

export interface Category {
  id: CategoryId;
  label: string;
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
export const SMART_CATEGORIES: readonly Category[] = [
  { id: RECENT_ID, label: "Last Played", smart: true },
  { id: FAVORITES_ID, label: "Favorites", smart: true },
];

export const BUILTIN_CATEGORIES: readonly Category[] = [
  { id: "games", label: "Games" },
  { id: "apps", label: "Apps" },
  { id: "homebrew", label: "Homebrew" },
];

/**
 * A built-in category with no titles is left out of the tab bar. Set to true to
 * show all three always. Categories the user created always show.
 */
export const SHOW_EMPTY_CATEGORIES = false;

export const CATEGORY_LABEL_MAX = 16;
export const CATEGORY_ID_PATTERN = /^[a-z0-9-]{1,32}$/;

/**
 * Category of an installed title, from its title id. Retail games use PCS ids,
 * system applications use NPXS ids, and everything else is homebrew.
 */
export function categoryOf(titleId: string): CategoryId {
  if (titleId.startsWith("PCS")) return "games";
  if (titleId.startsWith("NPXS")) return "apps";
  return "homebrew";
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
    const raw = JSON.parse(readFileSync(CATEGORIES_FILE, "utf8")) as unknown;
    // Earlier versions saved just the list of custom categories.
    const object = (Array.isArray(raw) ? { custom: raw } : raw) as {
      custom?: Partial<Category>[];
      order?: unknown;
      hidden?: unknown;
    };
    const seen = new Set([...SMART_CATEGORIES, ...BUILTIN_CATEGORIES].map((item) => item.id));
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
