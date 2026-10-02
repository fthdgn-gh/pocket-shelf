import { batch, createSignal } from "solid-js";
import { listArt, listBackdrops } from "./art-files.ts";
import { fill, type Messages } from "./i18n.ts";
import { getText, netAvailable, saveFile } from "./net.ts";
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
export type ScrapeStep = "setup" | "running" | "done";

/** A set of titles the user can fetch artwork for: all of them, or one category. */
export interface ScrapeScope {
  id: string;
  label: string;
  titles: Game[];
}

/** Rows of the setup step: the scope, missing pictures or all of them, then "Start". */
const SETUP_ROWS = 3;
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
 * is searched on SteamGridDB by its name; the first game found gives its
 * first icon and first backdrop. One request runs at a time.
 */
export function createScrapeFlow(deps: Deps) {
  const t = deps.t;
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
  /** The titles of the chosen scope the run would fetch for. */
  const wanted = (): Game[] => (scope()?.titles ?? []).filter((game) => kindsFor(game).length > 0);

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
    search(game);
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
      const file = assetFile(asset);
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
        assetPath(kind, asset),
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
    if (!loadKey()) {
      deps.askKey();
      return;
    }
    turn++;
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
    } else if (row() === 1) {
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
    } else if (step() === "done") setOpen(false);
  };

  /** Stop a run; from the other steps, close. What was fetched stays. */
  const cancelStep = () => {
    if (step() === "running") halt(t().scrapeStopped);
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
