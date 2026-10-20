import { batch, createSignal } from "solid-js";
import { listArt, listBackdrops } from "./art-files.ts";
import { fill, type Messages } from "./i18n.ts";
import { libretroFolder, matchName } from "./libretro.ts";
import { getText, netAvailable, saveFile } from "./net.ts";
import {
  SCRAPE_SOURCES,
  downloadCandidate,
  libretroCandidates,
  scraperFor,
  type Candidate,
  type Listings,
  type ScrapeSource,
} from "./scrapers.ts";
import {
  assetFile,
  assetPath,
  assetsUrl,
  authorization,
  loadKey,
  parseAssets,
  parseGames,
  saveKey,
  searchUrl,
  type AssetKind,
  type SgdbProblem,
} from "./steamgriddb.ts";
import type { Game } from "./types.ts";

/**
 * Where the flow is:
 *  - "setup": choosing which titles to fetch artwork for;
 *  - "running": going through them one at a time;
 *  - "done": finished, stopped, or halted by a problem.
 */
/**
 * The drawer's pages: choosing what to fetch, asking for the API key when
 * there is none (as the editor's SteamGridDB page does), the run, its result.
 */
export type ScrapeStep = "setup" | "key" | "running" | "done";

/** A set of titles the user can fetch artwork for: all of them, or one category. */
export interface ScrapeScope {
  id: string;
  label: string;
  titles: Game[];
}

/** Rows of the setup step: the scope, the source, missing pictures or all of them, then "Start". */
const SETUP_ROWS = 4;
export const SOURCE_ROW = 1;
export const MODE_ROW = 2;
export const START_ROW = SETUP_ROWS - 1;

/** Frames to wait before asking again after SteamGridDB said it is busy (5 s at 60 per second). */
const BUSY_WAIT = 300;

/** How many times in a row a busy reply is waited out before the run halts. */
const BUSY_RETRIES = 5;

interface Deps {
  /** The texts of the chosen language. */
  t: () => Messages;
  /** The scopes to choose from, and the index of the one to start on. */
  scopes: () => ScrapeScope[];
  startScope: () => number;
  /** Open the keyboard for the API key. */
  askKey: () => void;
  /** Use a downloaded file for a title. */
  apply: (titleId: string, kind: AssetKind, file: string) => void;
  /** The name a title came with (a RetroArch game's playlist label), which libretro matches. */
  label: (titleId: string) => string;
  /** libretro's box art names per console. */
  listings: Listings;
  /** The user's choice of scraper, and setting it from the Source row. */
  source: () => ScrapeSource;
  setSource: (source: ScrapeSource) => void;
}

/** Which pictures a title has no file of its own for. */
export function missingArt(game: Game): AssetKind[] {
  const kinds: AssetKind[] = [];
  // An icon the user chose, or one matched by file name, is kept.
  if (!game.art && !game.artIcon) kinds.push("icon");
  if (game.backdrop === undefined) kinds.push("backdrop");
  return kinds;
}

/**
 * Fetching artwork for many titles (SELECT menu -> Fetch artwork). Each title
 * goes to the scraper its source gives it (`scraperFor`). On SteamGridDB it is
 * searched by its name, and the first game found gives its first icon and
 * first backdrop. On libretro's server a RetroArch game's name is matched in
 * its console's box art list (`matchName`), and the box art (else the title
 * screen) becomes the icon and the screenshot (else the title screen) the
 * backdrop. One request runs at a time.
 */
export function createScrapeFlow(deps: Deps) {
  const t = deps.t;
  const source = deps.source;
  const [open, setOpen] = createSignal(false);
  const [step, setStep] = createSignal<ScrapeStep>("setup");
  const [row, setRow] = createSignal(0);
  const [scopeIndex, setScopeIndex] = createSignal(0);
  // Fetch both pictures for every title of the scope, replacing what it has.
  const [replace, setReplace] = createSignal(false);
  const [status, setStatus] = createSignal("");
  const [current, setCurrent] = createSignal("");
  const [total, setTotal] = createSignal(0);
  const [done, setDone] = createSignal(0);
  const [found, setFound] = createSignal<Record<AssetKind, number>>({ icon: 0, backdrop: 0 });
  const [missing, setMissing] = createSignal(0);

  const scope = (): ScrapeScope | undefined => deps.scopes()[scopeIndex()];
  /** The pictures to fetch for a title: the ones it lacks, or both when replacing. */
  const kindsFor = (game: Game): AssetKind[] => (replace() ? ["icon", "backdrop"] : missingArt(game));
  /** The titles of the chosen scope the run would fetch for: those the source has a scraper for. */
  const wanted = (): Game[] =>
    (scope()?.titles ?? []).filter((game) => kindsFor(game).length > 0 && scraperFor(source(), game) !== null);
  // Consoles libretro's server has no folder for, found during this run.
  const unknownFolders = new Set<string>();

  // The request in flight is cancelled when the run stops. `turn` also lets a
  // late callback notice that it no longer belongs to the run.
  let cancel: (() => void) | undefined;
  let turn = 0;
  let queue: Game[] = [];
  // Frames left before the title that got a busy reply is asked for again.
  let waitFrames = 0;
  let busyReplies = 0;
  let retry: (() => void) | undefined;
  // Files already in the art and backdrops folders; these are not downloaded again.
  const onDisk: Record<AssetKind, Set<string>> = { icon: new Set(), backdrop: new Set() };

  const halt = (message: string) => {
    turn++;
    cancel?.();
    cancel = undefined;
    retry = undefined;
    waitFrames = 0;
    batch(() => {
      setStep("done");
      setCurrent("");
      setStatus(message);
    });
  };

  /**
   * What to do about a failed reply: wait and ask again when the server is
   * busy, halt for everything else. A problem here would repeat for every
   * title that follows.
   */
  const problem = (reply: SgdbProblem, again: () => void) => {
    if (reply.problem !== "serverBusy" || busyReplies >= BUSY_RETRIES) {
      halt(fill(t()[reply.problem], { status: reply.status }));
      return;
    }
    busyReplies++;
    retry = again;
    waitFrames = BUSY_WAIT;
    setStatus(t().scrapeWaiting);
  };

  const next = () => {
    const game = queue.shift();
    if (!game) {
      halt(t().scrapeDone);
      return;
    }
    setCurrent(game.title);
    if (scraperFor(source(), game) === "libretro") libretro(game);
    else search(game);
  };

  /** libretro: match the game in its console's box art list, then fetch its pictures. */
  const libretro = (game: Game) => {
    const folder = libretroFolder(game) ?? "";
    const notFound = () => {
      setMissing((count) => count + 1);
      finish();
    };
    if (unknownFolders.has(folder)) {
      notFound();
      return;
    }
    const mine = turn;
    setStatus(t().gettingList);
    cancel = deps.listings.load(folder, (listing, error) => {
      if (mine !== turn) return;
      if (!listing) {
        // A server reply without a list (no such console there) skips the
        // console; a network failure would repeat for every title.
        if (error?.startsWith("HTTP")) {
          unknownFolders.add(folder);
          notFound();
        } else halt(fill(t().unreachableLibretro, { error: error ?? "" }));
        return;
      }
      const name = matchName(deps.label(game.id) || game.title, listing) ?? matchName(game.title, listing);
      if (!name) {
        notFound();
        return;
      }
      const candidates = libretroCandidates(folder, name, game);
      fetchCandidates(game, kindsFor(game).map((kind) => [kind, candidates[kind]] as const));
    });
  };

  /**
   * Fetch the first candidate of each kind the server has, trying the next one
   * on a 404, then move to the next title.
   */
  const fetchCandidates = (game: Game, wanted: readonly (readonly [AssetKind, readonly Candidate[]])[]) => {
    const [first, ...rest] = wanted;
    if (!first) {
      finish();
      return;
    }
    const [kind, candidates] = first;
    const [candidate, ...others] = candidates;
    if (!candidate) {
      fetchCandidates(game, rest);
      return;
    }
    const mine = turn;
    const use = () => {
      deps.apply(game.id, kind, candidate.file);
      setFound((counts) => ({ ...counts, [kind]: counts[kind] + 1 }));
      fetchCandidates(game, rest);
    };
    if (onDisk[kind].has(candidate.file)) {
      use();
      return;
    }
    setStatus(t().downloading);
    cancel = downloadCandidate(
      kind,
      candidate,
      (saved) => {
        if (mine !== turn) return;
        if (!saved.ok) {
          halt(fill(t().downloadFailed, { error: saved.error }));
          return;
        }
        if (saved.status !== 200) {
          fetchCandidates(game, [[kind, others], ...rest]);
          return;
        }
        onDisk[kind].add(candidate.file);
        use();
      },
      (received, size) => {
        if (mine !== turn || size <= 0) return;
        setStatus(fill(t().downloadingPercent, { percent: Math.round((received * 100) / size) }));
      },
    );
  };

  const finish = () => {
    setDone((count) => count + 1);
    next();
  };

  const search = (game: Game) => {
    const mine = turn;
    setStatus(t().searching);
    cancel = getText(searchUrl(game.title), authorization(loadKey()), (result) => {
      if (mine !== turn) return;
      if (!result.ok) {
        halt(fill(t().unreachable, { error: result.error }));
        return;
      }
      const games = parseGames(result.status, result.text);
      if (!Array.isArray(games)) {
        problem(games, () => search(game));
        return;
      }
      busyReplies = 0;
      const match = games[0];
      if (!match) {
        setMissing((count) => count + 1);
        finish();
        return;
      }
      fetchKinds(game, match.id, kindsFor(game));
    });
  };

  /** Fetch the first candidate of each kind in turn, then move to the next title. */
  const fetchKinds = (game: Game, gameId: number, kinds: AssetKind[]) => {
    const [kind, ...rest] = kinds;
    if (!kind) {
      finish();
      return;
    }
    const mine = turn;
    const after = () => fetchKinds(game, gameId, rest);
    setStatus(t().loadingArt);
    cancel = getText(assetsUrl(kind, gameId), authorization(loadKey()), (result) => {
      if (mine !== turn) return;
      if (!result.ok) {
        halt(fill(t().unreachable, { error: result.error }));
        return;
      }
      const assets = parseAssets(result.status, result.text);
      if (!Array.isArray(assets)) {
        problem(assets, () => fetchKinds(game, gameId, kinds));
        return;
      }
      busyReplies = 0;
      const asset = assets[0];
      if (!asset) {
        after();
        return;
      }
      const file = assetFile(asset, game);
      const use = () => {
        deps.apply(game.id, kind, file);
        setFound((counts) => ({ ...counts, [kind]: counts[kind] + 1 }));
        after();
      };
      if (onDisk[kind].has(file)) {
        use();
        return;
      }
      setStatus(t().downloading);
      cancel = saveFile(
        asset.url,
        assetPath(kind, asset, game),
        (saved) => {
          if (mine !== turn) return;
          if (!saved.ok) {
            halt(fill(t().downloadFailed, { error: saved.error }));
            return;
          }
          // A file the server no longer has is skipped; the title keeps what it had.
          if (saved.status !== 200) {
            after();
            return;
          }
          onDisk[kind].add(file);
          use();
        },
        (received, size) => {
          if (mine !== turn || size <= 0) return;
          setStatus(fill(t().downloadingPercent, { percent: Math.round((received * 100) / size) }));
        },
      );
    });
  };

  const run = () => {
    const titles = wanted();
    if (titles.length === 0) {
      setStatus(t().scrapeNothing);
      return;
    }
    if (!netAvailable()) {
      setStatus(t().noNetwork);
      return;
    }
    // The key is asked for only when some title goes to SteamGridDB.
    if (!loadKey() && titles.some((game) => scraperFor(source(), game) === "steamgriddb")) {
      batch(() => {
        setStep("key");
        setStatus("");
      });
      return;
    }
    turn++;
    unknownFolders.clear();
    queue = [...titles];
    busyReplies = 0;
    onDisk.icon = new Set(listArt());
    onDisk.backdrop = new Set(listBackdrops());
    batch(() => {
      setStep("running");
      setTotal(titles.length);
      setDone(0);
      setFound({ icon: 0, backdrop: 0 });
      setMissing(0);
      setStatus("");
    });
    next();
  };

  const start = () => {
    batch(() => {
      setOpen(true);
      setStep("setup");
      setRow(0);
      setScopeIndex(deps.startScope());
      setReplace(false);
      setStatus("");
    });
  };

  const move = (dx: number, dy: number) => {
    if (step() !== "setup") return;
    if (dy !== 0) setRow((value) => (value + dy + SETUP_ROWS) % SETUP_ROWS);
    else if (row() === 0) {
      const count = deps.scopes().length;
      batch(() => {
        setScopeIndex((value) => (value + dx + count) % count);
        setStatus("");
      });
    } else if (row() === SOURCE_ROW) {
      const count = SCRAPE_SOURCES.length;
      batch(() => {
        deps.setSource(SCRAPE_SOURCES[(SCRAPE_SOURCES.indexOf(source()) + dx + count) % count]!);
        setStatus("");
      });
    } else if (row() === MODE_ROW) {
      batch(() => {
        setReplace((on) => !on);
        setStatus("");
      });
    }
  };

  const confirm = () => {
    if (step() === "setup") {
      if (row() === START_ROW) run();
      else move(1, 0);
    } else if (step() === "key") deps.askKey();
    else if (step() === "done") setOpen(false);
  };

  /** Stop a run; from the key page, back to the setup; else close. What was fetched stays. */
  const cancelStep = () => {
    if (step() === "running") halt(t().scrapeStopped);
    else if (step() === "key") setStep("setup");
    else setOpen(false);
  };

  /** Call once per frame. */
  const frame = () => {
    if (waitFrames > 0 && --waitFrames === 0) {
      const again = retry;
      retry = undefined;
      again?.();
    }
  };

  /** The keyboard's result for the API key. */
  const submitKey = (text: string) => {
    if (saveKey(text)) run();
    else setStatus(t().badKey);
  };

  return {
    open,
    step,
    row,
    scope,
    replace,
    source,
    wanted,
    status,
    current,
    total,
    done,
    found,
    missing,
    start,
    move,
    confirm,
    cancel: cancelStep,
    frame,
    submitKey,
  };
}

export type ScrapeFlow = ReturnType<typeof createScrapeFlow>;
