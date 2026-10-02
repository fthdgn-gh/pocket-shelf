import { batch, createSignal } from "solid-js";
import { registerTexture } from "@pocketjs/framework";
import { rmSync } from "@pocketjs/framework/fs";
import {
  TEXTURE_PENDING,
  acquireBackdrop,
  artTexture,
  listArt,
  listBackdrops,
  releaseBackdrop,
} from "./art-files.ts";
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
  type SgdbAsset,
  type SgdbGame,
  type SgdbProblem,
} from "./steamgriddb.ts";
import type { Game } from "./types.ts";

/** Search rows visible at once in the games list, the search row included. */
export const ONLINE_ROWS = 6;

/**
 * Where the flow is:
 *  - "key": no usable API key yet;
 *  - "games": the search term and the games it found;
 *  - "assets": the chosen game's icons and backdrops, one candidate at a time.
 */
export type OnlineStep = "key" | "games" | "assets";

export const ASSET_KINDS: readonly AssetKind[] = ["icon", "backdrop"];

interface Deps {
  /** The texts of the chosen language. */
  t: () => Messages;
  /** The title being edited. */
  game: () => Game | undefined;
  /** Open the keyboard for the search term or the API key. */
  askText: (target: "search" | "apiKey", text: string) => void;
  /** Use a downloaded file for the edited title. */
  apply: (kind: AssetKind, file: string) => void;
  /** Whether any title uses this file, so it is kept when the flow closes. */
  inUse: (kind: AssetKind, file: string) => boolean;
}

/** Finding an icon and a backdrop for a title on SteamGridDB (title editor -> SteamGridDB). */
export function createOnlineFlow(deps: Deps) {
  const [open, setOpen] = createSignal(false);
  const [step, setStep] = createSignal<OnlineStep>("games");
  const [row, setRow] = createSignal(0);
  const [status, setStatus] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [term, setTerm] = createSignal("");
  const [games, setGames] = createSignal<SgdbGame[]>([]);
  const [chosen, setChosen] = createSignal<SgdbGame | undefined>(undefined);
  const [assets, setAssets] = createSignal<Record<AssetKind, SgdbAsset[]>>({ icon: [], backdrop: [] });
  const [index, setIndex] = createSignal<Record<AssetKind, number>>({ icon: 0, backdrop: 0 });
  /** Texture key of the candidate on screen, per kind; undefined while it loads. */
  const [preview, setPreview] = createSignal<Partial<Record<AssetKind, string>>>({});

  // Requests in flight are cancelled when the flow moves on. `turn` also lets
  // a late callback notice that it no longer belongs to the screen.
  let cancels: (() => void)[] = [];
  let turn = 0;
  // A backdrop candidate the host is still decoding: asked for again each frame.
  let retry: (() => void) | undefined;
  const cancelAll = () => {
    turn++;
    retry = undefined;
    for (const cancel of cancels) cancel();
    cancels = [];
    setBusy(false);
  };

  // Files on disk per kind: what earlier sessions kept, plus this session's downloads.
  const onDisk: Record<AssetKind, Set<string>> = { icon: new Set(), backdrop: new Set() };
  const downloaded: Record<AssetKind, Set<string>> = { icon: new Set(), backdrop: new Set() };

  // Texture handle of the backdrop candidate on screen, given back when it leaves.
  let backdropHandle = -1;
  const dropBackdropPreview = () => {
    releaseBackdrop(backdropHandle);
    backdropHandle = -1;
  };

  const t = deps.t;
  const problemText = (found: SgdbProblem) => fill(t()[found.problem], { status: found.status });
  const unreachable = (error: string) => fill(t().unreachable, { error });

  const kindOfRow = (): AssetKind => ASSET_KINDS[row()] ?? "icon";
  const current = (kind: AssetKind): SgdbAsset | undefined => assets()[kind][index()[kind]];

  const search = () => {
    const key = loadKey();
    if (!key) {
      batch(() => {
        setStep("key");
        setRow(0);
        setStatus("");
      });
      return;
    }
    cancelAll();
    const mine = turn;
    batch(() => {
      setStep("games");
      setGames([]);
      setRow(0);
      setBusy(true);
      setStatus(t().searching);
    });
    cancels.push(
      getText(searchUrl(term()), authorization(key), (result) => {
        if (mine !== turn) return;
        setBusy(false);
        if (!result.ok) {
          setStatus(unreachable(result.error));
          return;
        }
        const found = parseGames(result.status, result.text);
        if (!Array.isArray(found)) {
          setStatus(problemText(found));
          if (result.status === 401) setStep("key");
          return;
        }
        batch(() => {
          setGames(found);
          setRow(found.length > 0 ? 1 : 0);
          setStatus(found.length > 0 ? "" : t().noGames);
        });
      }),
    );
  };

  const loadAssets = (game: SgdbGame) => {
    const key = loadKey();
    cancelAll();
    const mine = turn;
    batch(() => {
      setChosen(game);
      setBusy(true);
      setStatus(t().loadingArt);
    });
    const lists: Partial<Record<AssetKind, SgdbAsset[]>> = {};
    let problem = "";
    for (const kind of ASSET_KINDS) {
      cancels.push(
        getText(assetsUrl(kind, game.id), authorization(key), (result) => {
          if (mine !== turn) return;
          const found = result.ok ? parseAssets(result.status, result.text) : unreachable(result.error);
          if (typeof found === "string") problem = found;
          else if (!Array.isArray(found)) problem = problemText(found);
          lists[kind] = Array.isArray(found) ? found : [];
          if (lists.icon === undefined || lists.backdrop === undefined) return;
          batch(() => {
            setBusy(false);
            setAssets({ icon: lists.icon ?? [], backdrop: lists.backdrop ?? [] });
            setIndex({ icon: 0, backdrop: 0 });
            setPreview({});
            dropBackdropPreview();
            setStep("assets");
            setRow(0);
            setStatus(problem);
          });
          if (!problem) show("icon");
        }),
      );
    }
  };

  /** Put the current candidate of `kind` on screen, downloading it first when needed. */
  const show = (kind: AssetKind) => {
    cancelAll();
    const mine = turn;
    const asset = current(kind);
    const id = deps.game()?.id;
    setPreview((previous) => ({ ...previous, [kind]: undefined }));
    if (kind === "backdrop") dropBackdropPreview();
    if (!asset || !id) {
      setStatus(kind === "icon" ? t().noIcons : t().noBackdrops);
      return;
    }
    const file = assetFile(asset);
    const display = () => {
      const handle = kind === "icon" ? artTexture(file) : acquireBackdrop(id, file);
      if (handle === TEXTURE_PENDING) {
        setStatus(t().loading);
        retry = display;
        return;
      }
      retry = undefined;
      if (handle < 0) {
        setStatus(t().unreadable);
        return;
      }
      if (kind === "backdrop") backdropHandle = handle;
      // The handle is part of the key: a picture that was freed and loaded
      // again comes back under a new handle.
      const key = `sgdb.${kind}.${file}.${handle}`;
      registerTexture(key, handle);
      batch(() => {
        setPreview((previous) => ({ ...previous, [kind]: key }));
        setStatus("");
      });
    };
    if (onDisk[kind].has(file)) {
      display();
      return;
    }
    batch(() => {
      setBusy(true);
      setStatus(t().downloading);
    });
    cancels.push(
      saveFile(
        asset.url,
        assetPath(kind, asset),
        (result) => {
          if (mine !== turn) return;
          setBusy(false);
          if (!result.ok || result.status !== 200) {
            const error = result.ok ? fill(t().statusCode, { status: result.status }) : result.error;
            setStatus(fill(t().downloadFailed, { error }));
            return;
          }
          onDisk[kind].add(file);
          downloaded[kind].add(file);
          display();
        },
        (received, total) => {
          if (mine !== turn) return;
          const kib = Math.round(received / 1024);
          setStatus(
            total > 0
              ? fill(t().downloadingPercent, { percent: Math.round((received * 100) / total) })
              : fill(t().downloadingSize, { size: kib }),
          );
        },
      ),
    );
  };

  const start = () => {
    const game = deps.game();
    if (!game) return;
    onDisk.icon = new Set(listArt());
    onDisk.backdrop = new Set(listBackdrops());
    downloaded.icon.clear();
    downloaded.backdrop.clear();
    batch(() => {
      setOpen(true);
      setTerm(game.title);
      setGames([]);
      setChosen(undefined);
      setPreview({});
      setRow(0);
      setStatus("");
    });
    if (!netAvailable()) {
      batch(() => {
        setStep("games");
        setStatus(t().noNetwork);
      });
      return;
    }
    search();
  };

  const close = () => {
    cancelAll();
    setPreview({});
    dropBackdropPreview();
    // Candidates that were looked at and not chosen are deleted.
    for (const kind of ASSET_KINDS) {
      for (const file of downloaded[kind]) {
        if (deps.inUse(kind, file)) continue;
        try {
          rmSync(`${kind === "icon" ? "art" : "backdrops"}/${file}`, { force: true });
        } catch {
          // The file stays; it shows up in the art picker.
        }
      }
      downloaded[kind].clear();
    }
    setOpen(false);
  };

  const move = (dx: number, dy: number) => {
    if (step() === "games" && dy !== 0) {
      const count = games().length + 1;
      setRow((value) => (value + dy + count) % count);
    } else if (step() === "assets" && dy !== 0) {
      setRow((value) => (value + dy + ASSET_KINDS.length) % ASSET_KINDS.length);
      if (preview()[kindOfRow()] === undefined) show(kindOfRow());
    } else if (step() === "assets" && dx !== 0) {
      const kind = kindOfRow();
      const count = assets()[kind].length;
      if (count < 2) return;
      setIndex((value) => ({ ...value, [kind]: (value[kind] + dx + count) % count }));
      show(kind);
    }
  };

  const confirm = () => {
    if (step() === "key") {
      deps.askText("apiKey", "");
    } else if (step() === "games") {
      if (row() === 0) deps.askText("search", term());
      else {
        const game = games()[row() - 1];
        if (game && !busy()) loadAssets(game);
      }
    } else {
      const kind = kindOfRow();
      const asset = current(kind);
      if (!asset || preview()[kind] === undefined) return;
      deps.apply(kind, assetFile(asset));
      setStatus(kind === "icon" ? t().iconApplied : t().backdropApplied);
    }
  };

  /** Back out one step; from the first step, close. */
  const cancel = () => {
    if (step() === "assets") {
      cancelAll();
      dropBackdropPreview();
      batch(() => {
        setPreview({});
        setStep("games");
        setRow(Math.max(1, games().findIndex((game) => game.id === chosen()?.id) + 1));
        setStatus("");
      });
    } else close();
  };

  /** Call once per frame. */
  const frame = () => retry?.();

  /** The keyboard's result for the search term. */
  const submitTerm = (text: string) => {
    if (!text) return;
    setTerm(text);
    search();
  };

  /** The keyboard's result for the API key. */
  const submitKey = (text: string) => {
    if (saveKey(text)) search();
    else setStatus(t().badKey);
  };

  return {
    open,
    step,
    row,
    status,
    busy,
    term,
    games,
    chosen,
    assets,
    index,
    preview,
    start,
    close,
    move,
    confirm,
    cancel,
    submitTerm,
    submitKey,
    frame,
  };
}

export type OnlineFlow = ReturnType<typeof createOnlineFlow>;
