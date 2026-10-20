import { readFileSync } from "@pocketjs/framework/fs";

// libretro's thumbnail server (https://thumbnails.libretro.com), the one
// RetroArch's "Download Thumbnails" uses. One folder per console, named after
// its database ("Nintendo - Game Boy"), each with Named_Boxarts, Named_Snaps
// and Named_Titles; a picture is `<name>.png`, the name in No-Intro's style
// ("Battleship (USA)"), with &*/:`"<>?\| written as _. No key, no account.
//
// A playlist's label is that name when RetroArch's scanner made it from a
// No-Intro set. Older GoodTools sets give "Battleship (U)" or "Frogger (UJ)
// [!]"; for those the console's list of box art names is fetched once
// (hosts/vita/src/http.rs keeps only the names, in
// `libretro/<console>.txt`) and a name is matched by its words and region.
// Measured on the user's 5,811 games: 510 by the exact name, 4,042 (70%) with
// the matching; most of the rest are hacks, pirate carts and translations,
// which have no box art.

export const SERVER = "https://thumbnails.libretro.com";

export type ThumbnailKind = "Named_Boxarts" | "Named_Snaps" | "Named_Titles";

/** Consoles whose playlist name is not the server's folder name. */
const FOLDERS: Record<string, string> = {
  // A playlist the user named, holding Neo Geo arcade sets.
  "Neo Geo": "SNK - Neo Geo",
};

/** The server's folder for a console's database name. Every MAME version shares "MAME". */
export function serverFolder(console: string): string {
  const name = console.trim();
  if (name.startsWith("MAME")) return "MAME";
  return FOLDERS[name] ?? name;
}

/**
 * The server's folder for a title: a RetroArch game's console, or the
 * PlayStation and PSP folders for PSX and PSP games. The Vita's folder holds a
 * handful of pictures, so Vita titles have none.
 */
export function libretroFolder(game: { console?: string; platform?: string }): string | undefined {
  if (game.console !== undefined) return serverFolder(game.console);
  if (game.platform === "psx") return "Sony - PlayStation";
  if (game.platform === "psp") return "Sony - PlayStation Portable";
  return undefined;
}

/** A label as a picture's file name, without `.png`: the characters RetroArch scrubs become _. */
export function scrubbed(label: string): string {
  return label.replace(/[&*/:`"<>?\\|]/g, "_");
}

/** URL of a picture, `name` being a file name from a listing or `scrubbed`. */
export function thumbnailUrl(folder: string, kind: ThumbnailKind, name: string): string {
  return `${SERVER}/${encodeURIComponent(folder)}/${kind}/${encodeURIComponent(name)}.png`;
}

/** URL of a console's box art listing page. */
export function listingUrl(folder: string): string {
  return `${SERVER}/${encodeURIComponent(folder)}/Named_Boxarts/`;
}

/** Where a console's box art names are kept under the data folder (the host checks the shape). */
export function listingPath(folder: string): string {
  const slug =
    folder
      .replace(/[^A-Za-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 56) || "console";
  return `libretro/${slug}.txt`;
}

/** A console's box art names, read from the data folder; null before the listing was fetched. */
export function readListing(folder: string): Listing | null {
  try {
    const names = readFileSync(listingPath(folder), "utf8")
      .split("\n")
      .filter((name) => name.length > 0);
    return names.length > 0 ? createListing(names) : null;
  } catch {
    return null;
  }
}

/** A console's names, with each one's base (see `baseName`) for matching. */
export interface Listing {
  names: ReadonlySet<string>;
  byBase: ReadonlyMap<string, readonly string[]>;
  /** Every name with its base, in listing order, for searching. */
  all: readonly (readonly [name: string, base: string])[];
}

export function createListing(names: readonly string[]): Listing {
  const byBase = new Map<string, string[]>();
  const all: [string, string][] = [];
  for (const name of names) {
    const base = baseName(name);
    all.push([name, base]);
    const list = byBase.get(base);
    if (list) list.push(name);
    else byBase.set(base, [name]);
  }
  return { names: new Set(names), byBase, all };
}

// Accents folded to plain letters, as search.ts does for the library.
const FOLD: Record<string, string> = {
  à: "a", á: "a", â: "a", ã: "a", ä: "a", å: "a", ç: "c", è: "e", é: "e", ê: "e", ë: "e",
  ì: "i", í: "i", î: "i", ï: "i", ñ: "n", ò: "o", ó: "o", ô: "o", õ: "o", ö: "o", ù: "u",
  ú: "u", û: "u", ü: "u", ý: "y", ÿ: "y", ß: "ss", ı: "i", ş: "s", ğ: "g",
};

/**
 * What two names are compared by: the name before its first bracket or
 * parenthesis, with ", The" moved to the front, accents folded, "&", "_" and
 * "and" dropped, and only letters and digits kept. "Adventures of Batman &
 * Robin, The (E)" and "Adventures of Batman _ Robin, The (USA)" give the same.
 */
export function baseName(name: string): string {
  let text = name.split(/\s*[([]/)[0]!.trim();
  text = text.replace(/^(.*), (The|A|An)$/i, "$2 $1").toLowerCase();
  text = [...text].map((letter) => FOLD[letter] ?? letter).join("");
  text = text.replace(/[&_]/g, " ").replace(/\band\b/g, " ");
  return text.replace(/[^a-z0-9]/g, "");
}

// GoodTools' region codes, as No-Intro writes the region.
const GOOD_TOOLS: Record<string, string> = {
  U: "USA", E: "Europe", J: "Japan", W: "World", UE: "USA", JU: "USA", UJ: "USA", JUE: "USA",
  A: "Australia", B: "Brazil", K: "Korea", F: "France", G: "Germany",
};
/** Regions preferred when a name says none, best first. */
const PREFERRED = ["USA", "World", "Europe", "Japan"];
/** Releases that are not the game's box: matched only by a label that is one too. */
const SPECIAL = /\((Beta|Proto|Sample|Demo|Hack|Pirate|Unl|Aftermarket)[^)]*\)/i;

/** The regions a name gives, in either style. */
export function regionsOf(name: string): string[] {
  const found: string[] = [];
  for (const [, tag] of name.matchAll(/\(([^)]*)\)/g)) {
    const code = GOOD_TOOLS[tag!];
    if (code) found.push(code);
    for (const part of tag!.split(",")) {
      const region = part.trim();
      if (PREFERRED.includes(region)) found.push(region);
    }
  }
  return found;
}

/** Of `names` that share a base, the one that best fits `label`'s region and kind. */
function bestOf(label: string, names: readonly string[]): string | undefined {
  const special = SPECIAL.test(label);
  const pool = names.filter((name) => special || !SPECIAL.test(name));
  const choices = pool.length > 0 ? pool : names;
  const wanted = [...regionsOf(label), ...PREFERRED];
  const rank = (name: string) => {
    const places = regionsOf(name).map((region) => wanted.indexOf(region)).filter((place) => place >= 0);
    return places.length > 0 ? Math.min(...places) : wanted.length;
  };
  let best: string | undefined;
  for (const name of choices) {
    if (best === undefined || rank(name) < rank(best) || (rank(name) === rank(best) && name.length < best.length)) {
      best = name;
    }
  }
  return best;
}

/** The listing's name for a playlist label: the label itself when the server has it, else the best match. */
export function matchName(label: string, listing: Listing): string | undefined {
  const exact = scrubbed(label);
  if (listing.names.has(exact)) return exact;
  const same = listing.byBase.get(baseName(label));
  return same ? bestOf(label, same) : undefined;
}

/**
 * Names for a typed term, best first, at most `max`: the names that match it
 * as `matchName` would, then those that hold every word of it, shorter first.
 */
export function searchNames(term: string, listing: Listing, max = 30): string[] {
  const base = baseName(term);
  if (!base) return [];
  const results: string[] = [];
  const same = listing.byBase.get(base) ?? [];
  const first = bestOf(term, same);
  if (first) results.push(first);
  for (const name of same) if (name !== first) results.push(name);
  const words = term
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((word) => word.length > 1 && word !== "the" && word !== "and");
  const more: string[] = [];
  for (const [name, nameBase] of listing.all) {
    if (nameBase === base) continue;
    if (words.length > 0 ? words.every((word) => nameBase.includes(word)) : nameBase.includes(base)) more.push(name);
  }
  more.sort((a, b) => a.length - b.length || a.localeCompare(b));
  return [...results, ...more].slice(0, max);
}
