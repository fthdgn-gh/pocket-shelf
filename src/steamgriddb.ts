import { readFileSync, writeFileSync } from "@pocketjs/framework/fs";
import { searchKey } from "./search.ts";

// SteamGridDB (https://www.steamgriddb.com/api/v2): search a game by name,
// then list its icons and heroes. Every call needs the user's own API key,
// sent as `Authorization: Bearer <key>`.
//
// The launcher decodes PNG only, so each list asks for static PNG files in
// sizes its decoders accept.

const API = "https://www.steamgriddb.com/api/v2";

/** Candidates kept per list. The API returns them best score first. */
export const ASSETS_MAX = 30;
export const GAMES_MAX = 12;

// ux0:/data/PocketShelf/steamgriddb.txt, one line: the API key.
const KEY_FILE = "steamgriddb.txt";
export const KEY_MAX = 64;
const KEY_PATTERN = /^[A-Za-z0-9]{16,64}$/;

export type AssetKind = "icon" | "backdrop";

export interface SgdbGame {
  id: number;
  name: string;
  /** Release year, when the database has one. */
  year?: number;
}

export interface SgdbAsset {
  id: number;
  url: string;
}

/** The saved API key, or "" when there is none or the file does not hold one. */
export function loadKey(): string {
  try {
    const key = readFileSync(KEY_FILE, "utf8").trim();
    return KEY_PATTERN.test(key) ? key : "";
  } catch {
    return "";
  }
}

/** Save a typed key. Returns false when it does not look like a key. */
export function saveKey(text: string): boolean {
  const key = text.trim();
  if (!KEY_PATTERN.test(key)) return false;
  try {
    writeFileSync(KEY_FILE, `${key}\n`);
    return true;
  } catch {
    return false;
  }
}

export function authorization(key: string): string {
  return `Bearer ${key}`;
}

export function searchUrl(term: string): string {
  return `${API}/search/autocomplete/${encodeURIComponent(term.trim())}`;
}

export function assetsUrl(kind: AssetKind, gameId: number): string {
  return kind === "icon"
    ? `${API}/icons/game/${gameId}?types=static&mimes=image/png&dimensions=128,256,512`
    : `${API}/heroes/game/${gameId}?types=static&mimes=image/png&dimensions=1920x620,1600x650`;
}

/**
 * A name as part of a file name: lowercase letters without their marks and
 * digits, with "-" between words ("Pokémon: Stadium 2" gives "pokemon-stadium-2").
 */
export function fileSlug(text: string): string {
  return searchKey(text)
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/** The title a downloaded picture is for. */
export interface AssetOwner {
  /** Title id, for example PCSA00069. */
  id: string;
}

/** File a candidate is downloaded to, under the data folder. */
export function assetPath(kind: AssetKind, asset: SgdbAsset, owner: AssetOwner): string {
  return `${kind === "icon" ? "art" : "backdrops"}/${assetFile(asset, owner)}`;
}

/**
 * File name of a candidate: the id of the title it is for, then the picture's
 * id on SteamGridDB, for example "PCSA00069-48213.png". The title id does not
 * change when the user renames the title, and it lets the picker list the
 * files of one title.
 */
export function assetFile(asset: SgdbAsset, owner: AssetOwner): string {
  return `${owner.id}-${asset.id}.png`;
}

/**
 * Whether a file is named after one of `names` (a title, its earlier title,
 * its id): the name alone, or followed by "-" and more.
 */
export function fileBelongsTo(file: string, names: readonly string[]): boolean {
  const base = fileSlug(file.replace(/\.png$/i, ""));
  return names.some((name) => {
    const slug = fileSlug(name);
    return slug !== "" && (base === slug || base.startsWith(`${slug}-`));
  });
}

/**
 * Why a reply has no data. `problem` is the key of its message in the
 * language files; the message for "serverStatus" names `status`.
 */
export interface SgdbProblem {
  problem: "keyRejected" | "serverBusy" | "serverStatus" | "badReply";
  status: number;
}

/** The `data` array of an API reply, or why there is none. */
function dataOf(status: number, text: string): unknown[] | SgdbProblem {
  if (status === 401) return { problem: "keyRejected", status };
  if (status === 404) return [];
  if (status === 429) return { problem: "serverBusy", status };
  if (status !== 200) return { problem: "serverStatus", status };
  try {
    const reply = JSON.parse(text) as { success?: boolean; data?: unknown };
    if (reply.success !== true || !Array.isArray(reply.data)) return { problem: "badReply", status };
    return reply.data;
  } catch {
    return { problem: "badReply", status };
  }
}

export function parseGames(status: number, text: string): SgdbGame[] | SgdbProblem {
  const data = dataOf(status, text);
  if (!Array.isArray(data)) return data;
  const games: SgdbGame[] = [];
  for (const item of data as { id?: unknown; name?: unknown; release_date?: unknown }[]) {
    if (typeof item?.id !== "number" || typeof item.name !== "string" || !item.name) continue;
    const seconds = typeof item.release_date === "number" ? item.release_date : 0;
    games.push({
      id: item.id,
      name: item.name,
      year: seconds > 0 ? new Date(seconds * 1000).getUTCFullYear() : undefined,
    });
    if (games.length === GAMES_MAX) break;
  }
  return games;
}

export function parseAssets(status: number, text: string): SgdbAsset[] | SgdbProblem {
  const data = dataOf(status, text);
  if (!Array.isArray(data)) return data;
  const assets: SgdbAsset[] = [];
  for (const item of data as { id?: unknown; url?: unknown }[]) {
    if (typeof item?.id !== "number" || typeof item.url !== "string") continue;
    // The host downloads over https and keeps PNG only.
    if (!item.url.startsWith("https://") || !/\.png$/i.test(item.url)) continue;
    assets.push({ id: item.id, url: item.url });
    if (assets.length === ASSETS_MAX) break;
  }
  return assets;
}
