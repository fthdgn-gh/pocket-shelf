import { batch, createEffect, createMemo, createSignal, on } from "solid-js";
import { registerTexture } from "@pocketjs/framework";
import { appIcon, launchApp } from "@pocketjs/framework/launcher";
import {
  BUILTIN_CATEGORIES,
  CATEGORY_LABEL_MAX,
  SHOW_EMPTY_CATEGORIES,
  cycleCategory,
  loadCategoryConfig,
  makeCategoryId,
  saveCategoryConfig,
  type Category,
} from "./categories.ts";
import {
  TEXTURE_PENDING,
  acquireBackdrop,
  artAccent,
  artTexture,
  listArt,
  listBackdrops,
  pumpBackdrops,
  releaseBackdrop,
} from "./art-files.ts";
import { boxColors, type BoxColors } from "./accent.ts";
import { pumpNet } from "./net.ts";
import { createOnlineFlow } from "./online.ts";
import { KEY_MAX } from "./steamgriddb.ts";
import { TINT_COLORS, type Catalog } from "./catalog.ts";
import { LETTER_ROWS, SYMBOL_ROWS, mapColumn } from "./keyboard.ts";
import {
  NO_BACKDROP,
  TITLE_MAX,
  USE_ICON,
  loadOverrides,
  saveOverrides,
  type Overrides,
  type TitleOverride,
} from "./overrides.ts";
import { loadSettings, saveSettings } from "./settings.ts";
import { DETAIL_LEVELS, VIEW_MODES, iconRadius, pageSize } from "./navigation.ts";
import { DEFAULT_FONT, FONTS, textClasses, type FontId } from "./text.ts";
import { DEFAULT_THEME, THEMES, dynamicTheme, themeById, type ThemeId } from "./themes.ts";
import type { CategoryId, ConfirmMode, DetailLevel, Game, ViewMode } from "./types.ts";

/** Rows in the SELECT menu: theme, font, view, details, backdrop, icon box, categories, confirm button. */
export const MENU_ROWS = 8;

/** Menu rows visible at once; the menu scrolls with the highlight. */
export const MENU_VISIBLE = 7;

/** Frames the selection rests on a title before its backdrop is loaded (0.25 s at 60 per second). */
const BACKDROP_DELAY = 15;

/** Frames between two requests for a backdrop the host is still decoding. */
const BACKDROP_RETRY = 2;

/** Frames between two requests for the icons the host is still decoding. */
const ICON_RETRY = 2;

/** Rows in the per-title editor: category, title, box art, backdrop, SteamGridDB, reset. */
export const EDITOR_ROWS = 6;

/** Art picker rows visible at once. */
export const PICKER_ROWS = 5;

export type Modal = "menu" | "editor" | "keyboard" | "art" | "online" | "categories";

/** What the file picker is choosing: a title's icon or its backdrop. */
export type PickerKind = "art" | "backdrop";

/** What the keyboard is typing into. */
type KeyboardTarget =
  | { kind: "title" }
  | { kind: "newCategory" }
  | { kind: "category"; id: string }
  | { kind: "search" }
  | { kind: "apiKey" };

function cycle<T>(list: readonly T[], current: T, delta: number): T {
  const index = list.indexOf(current);
  return list[(index + delta + list.length) % list.length];
}

/** All launcher state. Views render it; input.ts and the menu change it. */
export function createLauncherState(catalog: Catalog) {
  const { native } = catalog;

  // The user's changes per title id, saved to titles.json.
  const [overrides, setOverrides] = createSignal<Overrides>(loadOverrides());
  createEffect(on(overrides, saveOverrides, { defer: true }));
  const baseOf = (id: string) => catalog.games.find((game) => game.id === id);

  // Art files matched by name: `<title id>.png`, then `<title>.png`, ignoring
  // case. A file the user picked in the editor wins over a match.
  const readArtIndex = () => new Map(listArt().map((name) => [name.toLowerCase(), name]));
  const [artIndex, setArtIndex] = createSignal(readArtIndex());
  const findArt = (...names: string[]) => {
    for (const name of names) {
      const found = artIndex().get(`${name}.png`.toLowerCase());
      if (found) return found;
    }
    return undefined;
  };

  // Built-in categories, then the ones the user created (categories.json).
  const savedCategories = loadCategoryConfig();
  const [customCategories, setCustomCategories] = createSignal<Category[]>(savedCategories.custom);
  const [categoryOrder, setCategoryOrder] = createSignal<string[]>(savedCategories.order);
  const [hiddenCategories, setHiddenCategories] = createSignal<string[]>(savedCategories.hidden);
  // Every category in tab order. Ids missing from the saved order (new ones)
  // follow at the end, built-ins first.
  const allCategories = createMemo<Category[]>(() => {
    const base = [...BUILTIN_CATEGORIES, ...customCategories()];
    const position = (id: string) => {
      const index = categoryOrder().indexOf(id);
      return index < 0 ? Number.MAX_SAFE_INTEGER : index;
    };
    return base
      .map((item, index) => ({ item, index }))
      .sort((a, b) => position(a.item.id) - position(b.item.id) || a.index - b.index)
      .map(({ item }) => item);
  });
  const isHidden = (id: string) => hiddenCategories().includes(id);
  createEffect(
    on(
      [customCategories, allCategories, hiddenCategories],
      ([custom, all, hidden]) =>
        saveCategoryConfig({ custom, order: all.map((item) => item.id), hidden }),
      { defer: true },
    ),
  );
  const categoryLabel = (id: string) => allCategories().find((item) => item.id === id)?.label ?? id;

  const allGames = createMemo<Game[]>(() =>
    catalog.games.map((game) => {
      const change = overrides()[game.id];
      const title = change?.title ?? game.title;
      const chosen = change?.art;
      const forceIcon = chosen === USE_ICON;
      const art = forceIcon ? undefined : (chosen ?? findArt(game.id, game.title, title));
      // A category that was deleted falls back to the title's own.
      const wanted = change?.category;
      const category = wanted && allCategories().some((item) => item.id === wanted) ? wanted : game.category;
      return {
        ...game,
        title,
        category,
        art,
        backdrop: change?.backdrop,
        artAuto: !forceIcon && chosen === undefined && art !== undefined,
        artIcon: forceIcon,
      };
    }),
  );
  const patchOverride = (id: string, patch: Partial<TitleOverride>) =>
    setOverrides((current) => {
      const next: TitleOverride = { ...current[id], ...patch };
      for (const key of Object.keys(next) as (keyof TitleOverride)[]) {
        if (next[key] === undefined) delete next[key];
      }
      const copy = { ...current };
      if (Object.keys(next).length === 0) delete copy[id];
      else copy[id] = next;
      return copy;
    });

  // Categories in the tab bar, and the titles of the current one.
  const categories = createMemo(() =>
    allCategories().filter(
      (item) =>
        !isHidden(item.id) &&
        (item.custom || SHOW_EMPTY_CATEGORIES || allGames().some((game) => game.category === item.id)),
    ),
  );
  const [categoryId, setCategoryId] = createSignal<CategoryId>(
    categories()[0]?.id ?? BUILTIN_CATEGORIES[0].id,
  );
  const games = createMemo(() => allGames().filter((game) => game.category === categoryId()));

  const [selectedIndex, setSelectedIndex] = createSignal(0);
  const [launchingTitle, setLaunchingTitle] = createSignal<string | null>(null);
  const saved = loadSettings();
  const [themeId, setThemeId] = createSignal<ThemeId>(saved?.theme ?? DEFAULT_THEME);
  const [font, setFont] = createSignal<FontId>(saved?.font ?? DEFAULT_FONT);
  const [detail, setDetail] = createSignal<DetailLevel>(saved?.detail ?? "normal");
  const [view, setViewMode] = createSignal<ViewMode>(saved?.view ?? "carousel");
  const [confirmMode, setConfirmMode] = createSignal<ConfirmMode>(saved?.confirm ?? "circle");
  const [backdropOn, setBackdropOn] = createSignal(saved?.backdrop ?? true);
  const [iconBoxOn, setIconBoxOn] = createSignal(saved?.iconBox ?? true);

  // The launcher process ends when a title starts, so the selection is saved
  // with the settings and restored on the next start.
  if (saved?.category && categories().some((item) => item.id === saved.category)) {
    setCategoryId(saved.category);
    const index = games().findIndex((game) => game.id === saved.title);
    if (index >= 0) setSelectedIndex(index);
  }
  const persist = () =>
    saveSettings({
      theme: themeId(),
      font: font(),
      view: view(),
      detail: detail(),
      confirm: confirmMode(),
      backdrop: backdropOn(),
      iconBox: iconBoxOn(),
      category: categoryId(),
      title: games()[selectedIndex()]?.id,
    });
  // Persist on change; the initial run is skipped so a fresh start writes nothing.
  createEffect(on([themeId, font, view, detail, confirmMode, backdropOn, iconBoxOn], persist, { defer: true }));
  const [menuOpen, setMenuOpen] = createSignal(false);
  const [menuRow, setMenuRow] = createSignal(0);

  // Icons are decoded by the host on request, so only items near the selection
  // ask for one. A custom art file wins over the installed title's own icon;
  // the host caches both per name.
  const [icons, setIcons] = createSignal<Record<string, string>>({});
  // Box colors per title id, for icons that have transparent parts.
  const [boxes, setBoxes] = createSignal<Record<string, BoxColors>>({});
  /** The box to draw behind a title's icon, when the setting is on and the icon needs one. */
  const iconBox = (id: string): BoxColors | undefined => (iconBoxOn() ? boxes()[id] : undefined);
  const loadedSource = new Map<string, string>();
  const registeredKeys = new Set<string>();
  // Whether an icon near the selection is still being decoded by the host.
  let iconsPending = false;
  let iconWait = 0;
  // The strongest color of each loaded icon, as 0xRRGGBB (Dynamic theme).
  const iconColors = new Map<string, number | undefined>();
  const loadIcon = (game: Game) => {
    const source = game.art ? `art:${game.art}` : "app";
    if (loadedSource.get(game.id) === source) return;
    let handle = -1;
    let key = "";
    if (game.art) {
      handle = artTexture(game.art);
      key = `art.${game.art}`;
    }
    if (handle === -1) {
      handle = appIcon(game.id);
      key = `installed.${game.id}`;
    }
    if (handle === TEXTURE_PENDING) {
      // The host is decoding it. The tile shows the title's initials until
      // `frame` asks again and gets the icon.
      iconsPending = true;
      return;
    }
    loadedSource.set(game.id, source);
    if (handle < 0) {
      setIcons(({ [game.id]: _removed, ...rest }) => rest);
      setBoxes(({ [game.id]: _removed, ...rest }) => rest);
      iconColors.delete(game.id);
      return;
    }
    const accent = artAccent(handle);
    iconColors.set(game.id, accent < 0 ? undefined : accent & 0xffffff);
    const colors = boxColors(accent);
    setBoxes(({ [game.id]: _previous, ...rest }) => (colors ? { ...rest, [game.id]: colors } : rest));
    if (!registeredKeys.has(key)) {
      registeredKeys.add(key);
      registerTexture(key, handle);
    }
    setIcons((current) => ({ ...current, [game.id]: key }));
  };
  const loadIconsAround = () => {
    const radius = iconRadius(view());
    const center = selectedIndex();
    const list = games();
    for (let i = Math.max(0, center - radius); i <= Math.min(list.length - 1, center + radius); i++) {
      loadIcon(list[i]);
    }
  };

  const select = (index: number) => {
    if (games().length === 0) return;
    setSelectedIndex(Math.max(0, Math.min(games().length - 1, index)));
    setLaunchingTitle(null);
    loadIconsAround();
  };

  /** Switch to the next or previous category; the selection starts over. */
  const changeCategory = (delta: number) => {
    const next = cycleCategory(categories().map((item) => item.id), categoryId(), delta);
    if (next === categoryId()) return;
    batch(() => {
      setCategoryId(next);
      setSelectedIndex(0);
      setLaunchingTitle(null);
    });
    loadIconsAround();
  };

  const page = (direction: 1 | -1) => select(selectedIndex() + direction * pageSize(view()));

  const launchSelected = () => {
    const game = games()[selectedIndex()];
    if (!game) return;
    setLaunchingTitle(game.title);
    persist();
    console.log(`Launch requested: ${game.id}`);
    if (native && !launchApp(game.id)) console.log(`Launch rejected: ${game.id}`);
  };

  const setView = (next: ViewMode) => {
    setViewMode(next);
    loadIconsAround();
  };

  const toggleConfirm = () => setConfirmMode((mode) => (mode === "circle" ? "cross" : "circle"));

  // The Dynamic theme takes its colors from the selected title: its icon's
  // color, or its tint when it has no icon. `dynamicColor` moves toward that
  // color a little each frame, so the screen blends from one title to the next.
  const [dynamicColor, setDynamicColor] = createSignal<[number, number, number] | undefined>(undefined);
  const titleColor = (): number | undefined => {
    const game = games()[selectedIndex()];
    if (!game) return undefined;
    return iconColors.get(game.id) ?? TINT_COLORS[game.tint % TINT_COLORS.length];
  };
  const blendDynamicColor = () => {
    if (themeId() !== "dynamic") return;
    const target = titleColor();
    if (target === undefined) return;
    const goal = [(target >> 16) & 255, (target >> 8) & 255, target & 255];
    const now = dynamicColor();
    if (!now) {
      setDynamicColor([goal[0], goal[1], goal[2]]);
      return;
    }
    if (now.every((value, index) => value === goal[index])) return;
    // A quarter of the way each frame, then snap once it is within a step.
    const next = now.map((value, index) => {
      const gap = goal[index] - value;
      return Math.abs(gap) < 3 ? goal[index] : value + gap * 0.25;
    });
    setDynamicColor([next[0], next[1], next[2]]);
  };
  const theme = createMemo(() => {
    const color = themeId() === "dynamic" ? dynamicColor() : undefined;
    if (!color) return themeById(themeId());
    return dynamicTheme((Math.round(color[0]) << 16) | (Math.round(color[1]) << 8) | Math.round(color[2]));
  });
  /** Text class per role in the chosen font. */
  const text = createMemo(() => textClasses(font()));
  /** The face buttons that confirm and cancel, as icon names. */
  const confirmButton = (): ConfirmMode => confirmMode();
  const cancelButton = (): ConfirmMode => (confirmMode() === "circle" ? "cross" : "circle");

  // The current tab can disappear (hidden or deleted): move to the first one.
  createEffect(() => {
    const tabs = categories();
    if (tabs.length > 0 && !tabs.some((item) => item.id === categoryId())) {
      batch(() => {
        setCategoryId(tabs[0].id);
        setSelectedIndex(0);
        setLaunchingTitle(null);
      });
    }
  });
  // --- Backdrop ------------------------------------------------------------
  // The selected title's full-screen picture. Decoding one takes the host a
  // moment, so it loads once the selection has rested on a title, and the
  // previous picture stays underneath while the new one fades in.
  const [backdrop, setBackdrop] = createSignal<string | undefined>(undefined);
  const [backdropUnder, setBackdropUnder] = createSignal<string | undefined>(undefined);
  // Texture handles of the two pictures on screen, given back when they leave it.
  let backdropHandle = -1;
  let backdropUnderHandle = -1;
  const clearBackdrop = () => {
    releaseBackdrop(backdropHandle);
    releaseBackdrop(backdropUnderHandle);
    backdropHandle = -1;
    backdropUnderHandle = -1;
    batch(() => {
      setBackdrop(undefined);
      setBackdropUnder(undefined);
    });
  };
  let backdropWait = 0;
  const selectedId = createMemo(() => games()[selectedIndex()]?.id);
  createEffect(() => {
    selectedId();
    // A backdrop the user just chose for the selected title loads too.
    games()[selectedIndex()]?.backdrop;
    if (backdropOn()) backdropWait = BACKDROP_DELAY;
    else {
      backdropWait = 0;
      clearBackdrop();
    }
  });
  const loadBackdrop = () => {
    const game = games()[selectedIndex()];
    const id = game?.id;
    const file = game?.backdrop;
    const handle = id && native && file !== NO_BACKDROP ? acquireBackdrop(id, file ?? "") : -1;
    if (handle === TEXTURE_PENDING) {
      // The host is decoding it. The picture on screen stays until it is ready.
      backdropWait = BACKDROP_RETRY;
      return;
    }
    if (handle < 0) {
      clearBackdrop();
      return;
    }
    // The handle is part of the key: a picture that was freed and loaded
    // again comes back under a new handle.
    const key = `backdrop.${id}.${file ?? ""}.${handle}`;
    if (key === backdrop()) {
      // Already on screen: this second claim on the handle is not needed.
      releaseBackdrop(handle);
      return;
    }
    registerTexture(key, handle);
    // The picture that was underneath leaves the screen; the current one goes under.
    releaseBackdrop(backdropUnderHandle);
    backdropUnderHandle = backdropHandle;
    backdropHandle = handle;
    batch(() => {
      setBackdropUnder(backdrop());
      setBackdrop(key);
    });
  };
  /** Call once per frame. */
  const frame = () => {
    pumpNet();
    pumpBackdrops();
    online.frame();
    if (iconsPending && ++iconWait >= ICON_RETRY) {
      iconWait = 0;
      iconsPending = false;
      loadIconsAround();
    }
    blendDynamicColor();
    if (backdropWait > 0 && --backdropWait === 0) loadBackdrop();
  };

  // --- SELECT menu -------------------------------------------------------
  const openMenu = () => setMenuOpen(true);
  const closeMenu = () => setMenuOpen(false);
  const toggleMenu = () => setMenuOpen((open) => !open);
  const menuMove = (delta: number) => setMenuRow((row) => (row + delta + MENU_ROWS) % MENU_ROWS);
  /** Step the highlighted menu row's value forward or back. */
  const menuChange = (delta: number) => {
    const row = menuRow();
    if (row === 0) setThemeId((id) => cycle(THEMES.map((item) => item.id), id, delta));
    else if (row === 1) setFont((id) => cycle(FONTS.map((item) => item.id), id, delta));
    else if (row === 2) setView(cycle(VIEW_MODES, view(), delta));
    else if (row === 3) setDetail(cycle(DETAIL_LEVELS, detail(), delta));
    else if (row === 4) setBackdropOn((on) => !on);
    else if (row === 5) setIconBoxOn((on) => !on);
    else if (row === 6) openCategoryManager();
    else toggleConfirm();
  };

  // --- Per-title editor (triangle) ---------------------------------------
  const [editorId, setEditorId] = createSignal<string | null>(null);
  const [editorRow, setEditorRow] = createSignal(0);
  const [editorNote, setEditorNote] = createSignal("");
  const editorGame = createMemo(() => allGames().find((game) => game.id === editorId()));
  const editorOpen = () => editorId() !== null;

  const openEditor = () => {
    const game = games()[selectedIndex()];
    if (!game) return;
    setArtIndex(readArtIndex());
    loadIconsAround();
    setEditorId(game.id);
    setEditorRow(0);
    setEditorNote("");
  };
  const closeEditor = () => {
    setEditorId(null);
    clampSelection();
  };

  /** Keep the selection inside the current category's list after it changed. */
  const clampSelection = () => {
    setSelectedIndex((index) => Math.max(0, Math.min(games().length - 1, index)));
    loadIconsAround();
  };

  /** Move the edited title to the next or previous category. The tab stays put. */
  const changeGameCategory = (delta: number) => {
    const game = editorGame();
    if (!game) return;
    const choices = allCategories()
      .filter((item) => !isHidden(item.id) || item.id === game.category)
      .map((item) => item.id);
    const next = cycle(choices, game.category, delta);
    patchOverride(game.id, { category: next === baseOf(game.id)?.category ? undefined : next });
    clampSelection();
  };

  const resetGame = () => {
    const game = editorGame();
    if (!game) return;
    setOverrides(({ [game.id]: _removed, ...rest }) => rest);
    clampSelection();
    setEditorNote("Reset to the defaults");
  };

  // --- Title keyboard ----------------------------------------------------
  const [keyboardOpen, setKeyboardOpen] = createSignal(false);
  const [keyboardText, setKeyboardText] = createSignal("");
  const [keyRow, setKeyRow] = createSignal(1);
  const [keyCol, setKeyCol] = createSignal(0);
  const [shift, setShift] = createSignal(false);
  const [symbols, setSymbols] = createSignal(false);
  const keyRows = () => (symbols() ? SYMBOL_ROWS : LETTER_ROWS);

  const [keyboardTarget, setKeyboardTarget] = createSignal<KeyboardTarget>({ kind: "title" });
  const keyboardLimit = () => {
    const kind = keyboardTarget().kind;
    if (kind === "apiKey") return KEY_MAX;
    return kind === "title" || kind === "search" ? TITLE_MAX : CATEGORY_LABEL_MAX;
  };

  const openKeyboard = (target: KeyboardTarget, text: string) => {
    setKeyboardTarget(target);
    setKeyboardText(text);
    setKeyRow(1);
    setKeyCol(0);
    setShift(false);
    setSymbols(false);
    setKeyboardOpen(true);
  };
  const closeKeyboard = () => setKeyboardOpen(false);
  const keyboardMove = (dx: number, dy: number) => {
    const rows = keyRows();
    const row = keyRow();
    if (dy !== 0) {
      const next = (row + dy + rows.length) % rows.length;
      setKeyCol(mapColumn(rows[row].length, rows[next].length, keyCol()));
      setKeyRow(next);
    }
    if (dx !== 0) {
      const count = rows[row].length;
      setKeyCol((col) => (col + dx + count) % count);
    }
  };
  const typeText = (text: string) =>
    setKeyboardText((current) => (current + text).slice(0, keyboardLimit()));
  const backspace = () => setKeyboardText((current) => current.slice(0, -1));
  const toggleShift = () => setShift((on) => !on);
  const toggleSymbols = () => setSymbols((on) => !on);
  const commitKeyboard = () => {
    const text = keyboardText().trim();
    const target = keyboardTarget();
    if (target.kind === "title") {
      const game = editorGame();
      if (game) {
        // Typing the original title back removes the override.
        patchOverride(game.id, { title: !text || text === baseOf(game.id)?.title ? undefined : text });
      }
    } else if (target.kind === "search") {
      online.submitTerm(text);
    } else if (target.kind === "apiKey") {
      online.submitKey(text);
    } else if (text) {
      const used = allCategories().some(
        (item) =>
          item.label.toLowerCase() === text.toLowerCase() &&
          (target.kind === "newCategory" || item.id !== target.id),
      );
      if (used) {
        setCatNote("There is already a category with that name.");
      } else if (target.kind === "newCategory") {
        const id = makeCategoryId(text, new Set(allCategories().map((item) => item.id)));
        setCustomCategories((list) => [...list, { id, label: text, custom: true }]);
        setCatRow(allCategories().length);
        setCatNote("");
      } else {
        setCustomCategories((list) =>
          list.map((item) => (item.id === target.id ? { ...item, label: text } : item)),
        );
        setCatNote("");
      }
    }
    setKeyboardOpen(false);
  };
  const keyboardPress = () => {
    const key = keyRows()[keyRow()][keyCol()];
    if (key.action === "char") {
      typeText(shift() ? key.upper : key.lower);
      setShift(false);
    } else if (key.action === "space") typeText(" ");
    else if (key.action === "delete") backspace();
    else if (key.action === "shift") toggleShift();
    else if (key.action === "symbols") toggleSymbols();
    else commitKeyboard();
  };

  // --- File picker: box art and backdrop ----------------------------------
  // Two fixed rows, then the PNG files of the folder. For box art the fixed
  // rows are "Default" and "Game icon"; for a backdrop, "Default" and "None".
  const [artOpen, setArtOpen] = createSignal(false);
  const [pickerKind, setPickerKind] = createSignal<PickerKind>("art");
  const [artFiles, setArtFiles] = createSignal<string[]>([]);
  const [artRow, setArtRow] = createSignal(0);
  const [artNote, setArtNote] = createSignal("");

  const openArtPicker = (kind: PickerKind = "art") => {
    const game = editorGame();
    setPickerKind(kind);
    if (kind === "art") {
      const files = listArt();
      setArtIndex(new Map(files.map((name) => [name.toLowerCase(), name])));
      setArtFiles(files);
      const current = game?.artAuto ? -1 : files.indexOf(game?.art ?? "");
      setArtRow(game?.artIcon ? 1 : current >= 0 ? current + 2 : 0);
    } else {
      const files = listBackdrops();
      setArtFiles(files);
      const current = files.indexOf(game?.backdrop ?? "");
      setArtRow(game?.backdrop === NO_BACKDROP ? 1 : current >= 0 ? current + 2 : 0);
    }
    setArtNote("");
    setArtOpen(true);
  };
  const closeArtPicker = () => setArtOpen(false);
  const artMove = (dy: number) => {
    const count = artFiles().length + 2;
    setArtRow((row) => (row + dy + count) % count);
  };
  const artConfirm = () => {
    const game = editorGame();
    if (!game) return;
    const row = artRow();
    if (pickerKind() === "backdrop") {
      // Row 0 goes back to the title's own picture, row 1 draws none.
      const name = row === 0 ? undefined : row === 1 ? NO_BACKDROP : artFiles()[row - 2];
      // The host decodes the file in the background. One it cannot read
      // leaves the title without a backdrop.
      patchOverride(game.id, { backdrop: name });
      setArtOpen(false);
      return;
    }
    // Row 0 goes back to a file matched by name (or the title's own icon),
    // row 1 always shows the title's own icon.
    const name = row === 0 ? undefined : row === 1 ? USE_ICON : artFiles()[row - 2];
    // The host decodes the file in the background. One it cannot read leaves
    // the title with its own icon.
    patchOverride(game.id, { art: name });
    loadIconsAround();
    setArtOpen(false);
  };

  // --- SteamGridDB (title editor -> SteamGridDB) ----------------------------
  const online = createOnlineFlow({
    game: editorGame,
    askText: (target, text) => openKeyboard({ kind: target }, text),
    apply: (kind, file) => {
      const game = editorGame();
      if (!game) return;
      if (kind === "icon") {
        // The new file has to be in the name index before it can be an icon.
        setArtIndex(readArtIndex());
        patchOverride(game.id, { art: file });
        loadIconsAround();
      } else patchOverride(game.id, { backdrop: file });
    },
    inUse: (kind, file) =>
      Object.values(overrides()).some((change) => (kind === "icon" ? change.art : change.backdrop) === file),
  });

  // --- Category manager (SELECT menu -> Categories) ------------------------
  const [catOpen, setCatOpen] = createSignal(false);
  const [catRow, setCatRow] = createSignal(0);
  const [catArmed, setCatArmed] = createSignal<string | null>(null);
  const [catNote, setCatNote] = createSignal("");

  const openCategoryManager = () => {
    setCatRow(0);
    setCatArmed(null);
    setCatNote("");
    setCatOpen(true);
  };
  const closeCategoryManager = () => setCatOpen(false);
  /** The category on the highlighted manager row (row 0 is "New category"). */
  const catItem = () => allCategories()[catRow() - 1];
  const catMove = (dy: number) => {
    const count = allCategories().length + 1;
    setCatRow((row) => (row + dy + count) % count);
    setCatArmed(null);
    setCatNote("");
  };
  const catNew = () => openKeyboard({ kind: "newCategory" }, "");
  const catConfirm = () => {
    const item = catItem();
    if (!item) catNew();
    else if (item.custom) openKeyboard({ kind: "category", id: item.id }, item.label);
    else setCatNote("Built-in categories keep their names.");
  };
  const deleteCategory = (id: string) => {
    setCustomCategories((list) => list.filter((item) => item.id !== id));
    setHiddenCategories((list) => list.filter((item) => item !== id));
    // Titles placed in it return to their own category.
    setOverrides((current) => {
      const next: Overrides = {};
      for (const [titleId, change] of Object.entries(current)) {
        const { category, ...rest } = change;
        if (category === id) {
          if (Object.keys(rest).length > 0) next[titleId] = rest;
        } else next[titleId] = change;
      }
      return next;
    });
    setCatRow((row) => Math.min(row, allCategories().length));
    clampSelection();
  };
  /** Press twice on the same row: the first press asks, the second deletes. */
  const catDelete = () => {
    const item = catItem();
    if (!item) return;
    if (!item.custom) {
      setCatNote("Built-in categories can't be deleted. Hide them instead.");
      return;
    }
    if (catArmed() === item.id) {
      deleteCategory(item.id);
      setCatArmed(null);
      setCatNote("Deleted");
    } else {
      setCatArmed(item.id);
      // The manager shows the square button in front of this note while a row is armed.
      setCatNote(`again to delete ${item.label}`);
    }
  };
  /** Show or hide the highlighted category in the tab bar. */
  const catToggleHidden = () => {
    const item = catItem();
    if (!item) return;
    setCatArmed(null);
    if (isHidden(item.id)) {
      setHiddenCategories((list) => list.filter((id) => id !== item.id));
      setCatNote(`${item.label} is shown`);
      return;
    }
    if (allCategories().filter((other) => !isHidden(other.id)).length <= 1) {
      setCatNote("At least one category has to stay visible.");
      return;
    }
    setHiddenCategories((list) => [...list, item.id]);
    setCatNote(`${item.label} is hidden`);
  };
  /** Move the highlighted category earlier or later in the tab order. */
  const catReorder = (delta: number) => {
    const list = allCategories();
    const index = catRow() - 1;
    const target = index + delta;
    if (index < 0 || target < 0 || target >= list.length) return;
    const order = list.map((item) => item.id);
    [order[index], order[target]] = [order[target], order[index]];
    setCategoryOrder(order);
    setCatRow(target + 1);
    setCatArmed(null);
    setCatNote("");
  };

  // --- Routing for whichever panel is on top ------------------------------
  const modal = (): Modal | null => {
    if (keyboardOpen()) return "keyboard";
    if (artOpen()) return "art";
    if (online.open()) return "online";
    if (editorOpen()) return "editor";
    if (catOpen()) return "categories";
    return menuOpen() ? "menu" : null;
  };

  const modalMove = (dx: number, dy: number) => {
    switch (modal()) {
      case "menu":
        if (dy !== 0) menuMove(dy);
        else menuChange(dx);
        break;
      case "editor":
        if (dy !== 0) setEditorRow((row) => (row + dy + EDITOR_ROWS) % EDITOR_ROWS);
        else if (editorRow() === 0) changeGameCategory(dx);
        break;
      case "keyboard":
        keyboardMove(dx, dy);
        break;
      case "art":
        if (dy !== 0) artMove(dy);
        break;
      case "online":
        online.move(dx, dy);
        break;
      case "categories":
        if (dy !== 0) catMove(dy);
        break;
    }
  };
  const modalConfirm = () => {
    switch (modal()) {
      case "menu":
        menuChange(1);
        break;
      case "editor": {
        const row = editorRow();
        if (row === 0) changeGameCategory(1);
        else if (row === 1) openKeyboard({ kind: "title" }, editorGame()?.title ?? "");
        else if (row === 2) openArtPicker("art");
        else if (row === 3) openArtPicker("backdrop");
        else if (row === 4) online.start();
        else resetGame();
        break;
      }
      case "keyboard":
        keyboardPress();
        break;
      case "art":
        artConfirm();
        break;
      case "online":
        online.confirm();
        break;
      case "categories":
        catConfirm();
        break;
    }
  };
  const modalCancel = () => {
    switch (modal()) {
      case "keyboard":
        closeKeyboard();
        break;
      case "art":
        closeArtPicker();
        break;
      case "online":
        online.cancel();
        break;
      case "editor":
        closeEditor();
        break;
      case "categories":
        closeCategoryManager();
        break;
      case "menu":
        closeMenu();
        break;
    }
  };

  loadIconsAround();

  return {
    games,
    categories,
    categoryId,
    changeCategory,
    native,
    selectedIndex,
    launchingTitle,
    font,
    detail,
    view,
    confirmMode,
    icons,
    theme,
    text,
    confirmButton,
    cancelButton,
    backdropOn,
    iconBoxOn,
    iconBox,
    backdrop,
    backdropUnder,
    frame,
    select,
    page,
    launchSelected,
    menuOpen,
    menuRow,
    openMenu,
    closeMenu,
    toggleMenu,
    menuMove,
    menuChange,
    modal,
    modalMove,
    modalConfirm,
    modalCancel,
    openEditor,
    editorOpen,
    editorGame,
    editorRow,
    editorNote,
    keyboardText,
    keyRow,
    keyCol,
    shift,
    symbols,
    keyRows,
    toggleShift,
    toggleSymbols,
    backspace,
    commitKeyboard,
    artFiles,
    artRow,
    artNote,
    pickerKind,
    online,
    allCategories,
    hasTitles: () => catalog.games.length > 0,
    customCategories,
    categoryLabel,
    catRow,
    catNote,
    catArmed,
    catNew,
    catDelete,
    catToggleHidden,
    catReorder,
    isHidden,
  };
}

export type LauncherState = ReturnType<typeof createLauncherState>;
