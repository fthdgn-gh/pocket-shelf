import { saveFile, saveIcon, type NetResult } from "./net.ts";
import {
  libretroFolder,
  listingPath,
  listingUrl,
  readListing,
  thumbnailUrl,
  type Listing,
  type ThumbnailKind,
} from "./libretro.ts";
import { assetFile, type AssetKind, type AssetOwner, type SgdbAsset } from "./steamgriddb.ts";

// The places artwork is fetched from: SteamGridDB (any title, with the user's
// API key) and libretro's thumbnail server (RetroArch games, no key). The
// title editor finds pictures for one title from either; Fetch artwork goes
// through many, each from the one its source setting gives it.

export type ScraperId = "steamgriddb" | "libretro";

export const SCRAPER_NAMES: Record<ScraperId, string> = {
  steamgriddb: "SteamGridDB",
  libretro: "libretro",
};

/**
 * The user's choice of scraper (setting `scraper`, shown in the title editor
 * and in Fetch artwork): Auto, the default, gives libretro to RetroArch games
 * and SteamGridDB to the rest; or one of them for every title.
 */
export type ScrapeSource = "auto" | ScraperId;
export const SCRAPE_SOURCES: readonly ScrapeSource[] = ["auto", "steamgriddb", "libretro"];
export const DEFAULT_SCRAPE_SOURCE: ScrapeSource = "auto";

/**
 * The scraper a title's pictures come from under a choice; null when the
 * choice is libretro and the server has no folder for the title (homebrew,
 * Vita and PSM titles, system applications).
 */
export function scraperFor(source: ScrapeSource, game: { console?: string; platform?: string }): ScraperId | null {
  if (source === "steamgriddb") return "steamgriddb";
  if (source === "libretro") return libretroFolder(game) === undefined ? null : "libretro";
  return game.console !== undefined ? "libretro" : "steamgriddb";
}

/** One picture a title could use. */
export interface Candidate {
  url: string;
  /** File name in `art/` (an icon) or `backdrops/` (a backdrop). */
  file: string;
  /** Saved fitted into a square icon by the host (a box art); else as it is. */
  fit: boolean;
}

export function candidatePath(kind: AssetKind, candidate: Candidate): string {
  return `${kind === "icon" ? "art" : "backdrops"}/${candidate.file}`;
}

/** Download a candidate to its file. Returns a function that cancels it. */
export function downloadCandidate(
  kind: AssetKind,
  candidate: Candidate,
  done: (result: NetResult) => void,
  progress?: (received: number, total: number) => void,
): () => void {
  const save = candidate.fit ? saveIcon : saveFile;
  return save(candidate.url, candidatePath(kind, candidate), done, progress);
}

/** SteamGridDB's pictures of a kind as candidates. */
export function sgdbCandidates(assets: readonly SgdbAsset[], owner: AssetOwner): Candidate[] {
  return assets.map((asset) => ({ url: asset.url, file: assetFile(asset, owner), fit: false }));
}

/**
 * A libretro name's pictures as candidates: the box art, then the title
 * screen, as the icon; the in-game screenshot, then the title screen, as the
 * backdrop. Files are `<title id>-lr-box.png` and so on, which SteamGridDB's
 * numeric picture ids never give.
 */
export function libretroCandidates(folder: string, name: string, owner: AssetOwner): Record<AssetKind, Candidate[]> {
  const make = (kind: ThumbnailKind, suffix: string, fit: boolean): Candidate => ({
    url: thumbnailUrl(folder, kind, name),
    file: `${owner.id}-lr-${suffix}.png`,
    fit,
  });
  return {
    icon: [make("Named_Boxarts", "box", true), make("Named_Titles", "title", true)],
    backdrop: [make("Named_Snaps", "snap", false), make("Named_Titles", "title", false)],
  };
}

/**
 * Consoles' box art names, read from the data folder, or fetched from the
 * server the first time a console is needed. A listing kept from an earlier
 * fetch is used as it is; a picture added to the server later is missed until
 * the file is deleted.
 */
export function createListings() {
  const loaded = new Map<string, Listing>();
  /**
   * Call `done` with a console's listing, fetching it when there is none yet.
   * Returns a function that cancels the fetch.
   */
  const load = (folder: string, done: (listing: Listing | null, error?: string) => void): (() => void) => {
    const known = loaded.get(folder) ?? readListing(folder);
    if (known) {
      loaded.set(folder, known);
      done(known);
      return () => {};
    }
    return saveFile(listingUrl(folder), listingPath(folder), (result) => {
      if (!result.ok) {
        done(null, result.error);
        return;
      }
      const listing = result.status === 200 ? readListing(folder) : null;
      if (listing) loaded.set(folder, listing);
      done(listing, listing ? undefined : `HTTP ${result.status}`);
    });
  };
  return { load };
}

export type Listings = ReturnType<typeof createListings>;
