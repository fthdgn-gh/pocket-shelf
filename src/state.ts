import { batch, createEffect, createMemo, createSignal, on } from "solid-js";
import { registerTexture } from "@pocketjs/framework";
import { rmSync } from "@pocketjs/framework/fs";
import { appIcon, launchApp } from "@pocketjs/framework/launcher";
import {
  BUILTIN_CATEGORIES,
  CATEGORY_LABEL_MAX,
  FAVORITES_ID,
  RECENT_ID,
  SEARCH_CATEGORY,
  SEARCH_ID,
  SHOW_EMPTY_CATEGORIES,
  SMART_CATEGORIES,
  SYSTEM_ID,
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
import { enablePlugin, pluginState, type PluginState } from "./adrenaline.ts";
import { pumpNet } from "./net.ts";
import { createOnlineFlow } from "./online.ts";
import { KEY_MAX, fileBelongsTo } from "./steamgriddb.ts";
import { TINT_COLORS, loadCatalog, scanProgress, startScan, type Catalog, type ScanProgress } from "./catalog.ts";
import { timed } from "./diagnostics.ts";
import { DEFAULT_LANGUAGE, LANGUAGES, MESSAGES, fill, type Language } from "./i18n.ts";
import { KEY_PAGES, LETTER_ROWS, SYMBOL_ROWS, accentRows, mapColumn, type KeyPage } from "./keyboard.ts";
import {
  NO_BACKDROP,
  TITLE_MAX,
  USE_ICON,
  filesInUse,
  loadOverrides,
  reloadOverrides,
  saveOverrides,
  withoutFiles,
  type Overrides,
  type TitleOverride,
} from "./overrides.ts";
import { loadRecent, pushRecent, saveRecent } from "./recent.ts";
import { createScrapeFlow, type ScrapeScope } from "./scrape.ts";
import { createUpdateFlow } from "./update-flow.ts";
import { DEFAULT_CHANNEL, UPDATE_CHANNELS, type UpdateChannel } from "./updates.ts";
import { searchTitles } from "./search.ts";
import { CLOCK_FORMATS, loadSettings, saveSettings } from "./settings.ts";
import { DETAIL_LEVELS, VIEW_MODES, iconRadius, pageSize } from "./navigation.ts";
import { DEFAULT_FONT, FONTS, textClasses, type FontId } from "./text.ts";
import { MENU, isMenuPage, menuPath, parentPage, type MenuItem, type MenuPage } from "./menu-items.ts";
import { readStatus, sameStatus, type Status } from "./status.ts";
import { DEFAULT_THEME, THEMES, dynamicTheme, themeById, type ThemeId } from "./themes.ts";
import type { CategoryId, ClockFormat, ConfirmMode, DetailLevel, Game, ViewMode } from "./types.ts";

export type { MenuItem, MenuPage };

/** Menu rows visible at once; the menu scrolls with the highlight. */
export const MENU_VISIBLE = 7;

/** Frames the selection rests on a title before its backdrop is loaded (0.25 s at 60 per second). */
const BACKDROP_DELAY = 15;

/** Frames between two requests for a backdrop the host is still decoding. */
const BACKDROP_RETRY = 2;

/** Frames the picker's highlight rests on a row before its picture is loaded. */
const PREVIEW_DELAY = 8;

/** Frames between two requests for a picker picture the host is still decoding. */
const PREVIEW_RETRY = 2;

/** Frames between two readings of the status bar (about one second). */
const STATUS_FRAMES = 60;

/** Frames between two requests for the icons the host is still decoding. */
const ICON_RETRY = 2;

/** Rows in the per-title editor: favorite, category, title, box art, backdrop, SteamGridDB, reset. */
export const EDITOR_ROWS = 7;

/** Art picker rows visible at once. */
export const PICKER_ROWS = 5;

export type Modal =
  | "adrenaline"
  | "menu"
  | "editor"
  | "keyboard"
  | "art"
  | "online"
  | "categories"
  | "scrape"
  | "clean"
  | "rescan"
  | "update"
  | "diagnostics";

/** What the file picker is choosing: a title's icon or its backdrop. */
export type PickerKind = "art" | "backdrop";

/** What the keyboard is typing into. */
type KeyboardTarget =
  | { kind: "title" }
  | { kind: "newCategory" }
  | { kind: "category"; id: string }
  | { kind: "search" }
  | { kind: "apiKey" }
  | { kind: "library" };

function cycle<T>(list: readonly T[], current: T, delta: number): T {
  const index = list.indexOf(current);
  return list[(index + delta + list.length) % list.length];
}

/** All launcher state. Views render it; input.ts and the menu change it. */
export function createLauncherState(catalog: Catalog) {
  const { native } = catalog;
  // The installed titles; replaced when the user scans again.
  const [catalogGames, setCatalogGames] = createSignal(catalog.games);
  const saved = timed("settings", loadSettings);

  // The launcher's own texts in the chosen language.
  const [language, setLanguage] = createSignal<Language>(saved?.language ?? DEFAULT_LANGUAGE);
  const t = createMemo(() => MESSAGES[language()]);

  // The user's changes per title id, saved to titles.json.
  const [overrides, setOverrides] = createSignal<Overrides>(timed("changes", loadOverrides));
  createEffect(on(overrides, saveOverrides, { defer: true }));
  const baseOf = (id: string) => catalogGames().find((game) => game.id === id);

  // Art files matched by name: `<title id>.png`, then `<title>.png`, ignoring
  // case. A file the user picked in the editor wins over a match.
  const readArtIndex = () => new Map(listArt().map((name) => [name.toLowerCase(), name]));
  const [artIndex, setArtIndex] = createSignal(timed("art", readArtIndex));
  const findArt = (...names: string[]) => {
    for (const name of names) {
      const found = artIndex().get(`${name}.png`.toLowerCase());
      if (found) return found;
    }
    return undefined;
  };

  // Smart and built-in categories, then the ones the user created (categories.json).
  const savedCategories = timed("categories", loadCategoryConfig);
  const [customCategories, setCustomCategories] = createSignal<Category[]>(savedCategories.custom);
  const [categoryOrder, setCategoryOrder] = createSignal<string[]>(savedCategories.order);
  const [hiddenCategories, setHiddenCategories] = createSignal<string[]>(savedCategories.hidden);
  // Every category in tab order. Of the ids missing from the saved order, the
  // smart ones go first and the others (new ones) follow at the end,
  // built-ins before custom.
  const allCategories = createMemo<Category[]>(() => {
    const base = [...SMART_CATEGORIES, ...BUILTIN_CATEGORIES, ...customCategories()];
    const position = (item: Category) => {
      const index = categoryOrder().indexOf(item.id);
      if (index >= 0) return index;
      return item.smart ? -1 : Number.MAX_SAFE_INTEGER;
    };
    return base
      .map((item, index) => ({ item, index }))
      .sort((a, b) => position(a.item) - position(b.item) || a.index - b.index)
      .map(({ item }) => (item.name ? { ...item, label: t().categories[item.name] } : item));
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
    catalogGames().map((game) => {
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
        favorite: change?.favorite === true,
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

  // Title ids in the order they were last started, newest first (recent.json).
  const [recent, setRecent] = createSignal<string[]>(timed("recent", loadRecent));
  // The term of the open search (square button); empty when no search is open.
  const [searchTerm, setSearchTerm] = createSignal("");
  /** The titles a category lists. A title that was uninstalled drops out of "Last Played". */
  const titlesOf = (id: CategoryId): Game[] => {
    if (id === SEARCH_ID) return searchTitles(allGames(), searchTerm());
    if (id === RECENT_ID) {
      const byId = new Map(allGames().map((game) => [game.id, game]));
      return recent().flatMap((titleId) => byId.get(titleId) ?? []);
    }
    if (id === FAVORITES_ID) return allGames().filter((game) => game.favorite);
    return allGames().filter((game) => game.category === id);
  };

  // Categories in the tab bar, and the titles of the current one. An open
  // search adds its tab in front, also when it found nothing.
  const categories = createMemo(() => {
    const tabs = allCategories().filter(
      (item) =>
        !isHidden(item.id) && (item.custom || SHOW_EMPTY_CATEGORIES || titlesOf(item.id).length > 0),
    );
    return searchTerm() ? [{ ...SEARCH_CATEGORY, label: t().categories.search }, ...tabs] : tabs;
  });
  const [categoryId, setCategoryId] = createSignal<CategoryId>(
    categories()[0]?.id ?? BUILTIN_CATEGORIES[0].id,
  );
  const games = createMemo(() => titlesOf(categoryId()));
  /** How many titles each category in the tab bar lists. */
  const categoryCounts = createMemo(
    () => new Map(categories().map((item) => [item.id, titlesOf(item.id).length])),
  );
  const categoryCount = (id: CategoryId) => categoryCounts().get(id) ?? 0;

  const [selectedIndex, setSelectedIndex] = createSignal(0);
  const [launchingTitle, setLaunchingTitle] = createSignal<string | null>(null);
  const [themeId, setThemeId] = createSignal<ThemeId>(saved?.theme ?? DEFAULT_THEME);
  const [font, setFont] = createSignal<FontId>(saved?.font ?? DEFAULT_FONT);
  const [detail, setDetail] = createSignal<DetailLevel>(saved?.detail ?? "normal");
  const [view, setViewMode] = createSignal<ViewMode>(saved?.view ?? "carousel");
  const [confirmMode, setConfirmMode] = createSignal<ConfirmMode>(saved?.confirm ?? "circle");
  const [backdropOn, setBackdropOn] = createSignal(saved?.backdrop ?? true);
  const [iconBoxOn, setIconBoxOn] = createSignal(saved?.iconBox ?? true);
  const [statusBarOn, setStatusBarOn] = createSignal(saved?.statusBar ?? true);
  const [clockFormat, setClockFormat] = createSignal<ClockFormat>(saved?.clock ?? "system");
  const [batteryPercentOn, setBatteryPercentOn] = createSignal(saved?.batteryPercent ?? true);
  const [updateChannel, setUpdateChannel] = createSignal<UpdateChannel>(saved?.updates ?? DEFAULT_CHANNEL);

  // The launcher process ends when a title starts, so the selection is saved
  // with the settings and restored on the next start.
  if (saved?.category && categories().some((item) => item.id === saved.category)) {
    setCategoryId(saved.category);
    const index = games().findIndex((game) => game.id === saved.title);
    if (index >= 0) setSelectedIndex(index);
  }
  // Where the selection was when the search opened; closing it goes back there.
  let beforeSearch: { category: CategoryId; title?: string } | undefined;
  const persist = () => {
    const game = games()[selectedIndex()];
    // A search does not outlive the process: the selected result is saved
    // under its own category.
    const category = categoryId() === SEARCH_ID ? (game?.category ?? beforeSearch?.category) : categoryId();
    saveSettings({
      language: language(),
      theme: themeId(),
      font: font(),
      view: view(),
      detail: detail(),
      confirm: confirmMode(),
      backdrop: backdropOn(),
      iconBox: iconBoxOn(),
      statusBar: statusBarOn(),
      clock: clockFormat(),
      batteryPercent: batteryPercentOn(),
      updates: updateChannel(),
      category,
      title: game?.id,
    });
  };
  // Persist on change; the initial run is skipped so a fresh start writes nothing.
  createEffect(
    on(
      [
        language,
        themeId,
        font,
        view,
        detail,
        confirmMode,
        backdropOn,
        iconBoxOn,
        statusBarOn,
        clockFormat,
        batteryPercentOn,
        updateChannel,
      ],
      persist,
      { defer: true },
    ),
  );
  const [menuOpen, setMenuOpen] = createSignal(false);
  const [menuRow, setMenuRow] = createSignal(0);
  const [menuPage, setMenuPage] = createSignal<MenuPage>("main");

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

  const page = (direction: 1 | -1) => select(selectedIndex() + direction * pageSize(view(), statusBarOn()));

  // --- Search (square) ----------------------------------------------------
  const searching = () => categoryId() === SEARCH_ID;
  /** Open the keyboard for a search term. On the search tab it starts from the current term. */
  const openSearch = () => openKeyboard({ kind: "library" }, searching() ? searchTerm() : "");
  /** Close the search and return to the title that was selected before it. */
  const clearSearch = () => {
    if (!searchTerm()) return;
    const back = beforeSearch;
    beforeSearch = undefined;
    if (searching()) {
      const tabs = categories().filter((item) => item.id !== SEARCH_ID);
      const category = tabs.find((item) => item.id === back?.category)?.id ?? tabs[0]?.id;
      if (category) {
        setCategoryId(category);
        setSelectedIndex(Math.max(0, games().findIndex((game) => game.id === back?.title)));
      }
    }
    setSearchTerm("");
    setLaunchingTitle(null);
    loadIconsAround();
  };
  /** Show the titles `term` finds in the search tab. An empty term closes the search. */
  const applySearch = (term: string) => {
    if (!term) {
      clearSearch();
      return;
    }
    if (!searching()) beforeSearch = { category: categoryId(), title: games()[selectedIndex()]?.id };
    batch(() => {
      setSearchTerm(term);
      setCategoryId(SEARCH_ID);
      setSelectedIndex(0);
      setLaunchingTitle(null);
    });
    loadIconsAround();
  };
  /** The cancel button with no panel open: leave the search tab. */
  const closeSearch = () => {
    if (searching()) clearSearch();
  };

  // --- Adrenaline's boot plugin --------------------------------------------
  // A title without a bubble starts through Adrenaline, with Pocket Shelf's
  // plugin loaded at its XMB. When the plugin is not on, a drawer says why
  // and offers to add it or turn it on; the plugin lists change only then.
  const [adrPlugin, setAdrPlugin] = createSignal<PluginState | null>(null);
  const [adrFailed, setAdrFailed] = createSignal(false);
  const closeAdrenaline = () => setAdrPlugin(null);
  const adrenalineConfirm = () => {
    const plugin = adrPlugin();
    if (plugin !== "missing" && plugin !== "off") {
      closeAdrenaline();
      return;
    }
    if (!enablePlugin()) {
      setAdrFailed(true);
      return;
    }
    closeAdrenaline();
    launchSelected();
  };

  const launchSelected = () => {
    const game = games()[selectedIndex()];
    if (!game) return;
    if (native && game.adrenaline) {
      const plugin = pluginState();
      if (plugin !== "on") {
        batch(() => {
          setAdrFailed(false);
          setAdrPlugin(plugin);
        });
        return;
      }
    }
    setLaunchingTitle(game.title);
    // The files are written before the request: the host ends this process
    // once this frame is on screen, and a process still alive when the system
    // takes the request gets the "close this application?" prompt. Two writes
    // after the request were enough for that on hardware.
    const before = recent();
    const list = pushRecent(before, game.id);
    saveRecent(list);
    persist();
    console.log(`Launch requested: ${game.id}`);
    if (native && !launchApp(game.id)) {
      console.log(`Launch rejected: ${game.id}`);
      saveRecent(before);
      return;
    }
    batch(() => {
      setRecent(list);
      // In "Last Played" the started title moves to the front; follow it.
      if (categoryId() === RECENT_ID) setSelectedIndex(0);
    });
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
    scrape.frame();
    update.frame();
    if (iconsPending && ++iconWait >= ICON_RETRY) {
      iconWait = 0;
      iconsPending = false;
      loadIconsAround();
    }
    blendDynamicColor();
    if (backdropWait > 0 && --backdropWait === 0) loadBackdrop();
    if (previewWait > 0 && --previewWait === 0) loadPickerPreview();
    if (statusBarOn() && ++statusWait >= STATUS_FRAMES) {
      statusWait = 0;
      setStatus(readStatus());
    }
    pumpRescan();
  };

  // --- Status bar ------------------------------------------------------------
  // Read on the first frame, then once a second while the bar is shown. An
  // unchanged reading does not update the bar.
  const [status, setStatus] = createSignal<Status | null>(null, { equals: sameStatus });
  let statusWait = STATUS_FRAMES - 1;

  // --- SELECT menu -------------------------------------------------------
  /** Show `page` with the row that opens `from` highlighted, or its first row. */
  const showMenuPage = (page: MenuPage, from?: MenuPage) => {
    setMenuPage(page);
    setMenuRow(from ? Math.max(0, MENU[page].indexOf(from)) : 0);
  };
  // The menu opens on its main page. From a group it opens on the group's row
  // there; from the main page, on the row it was left at.
  const openMenu = () => {
    const top = menuPath(menuPage())[1] ?? menuPage();
    if (menuPage() !== "main") showMenuPage("main", top);
    setMenuOpen(true);
  };
  const closeMenu = () => setMenuOpen(false);
  const toggleMenu = () => (menuOpen() ? closeMenu() : openMenu());
  /** Back to the page holding this one, on its row; the main page closes the menu. */
  const menuBack = () => {
    const parent = parentPage(menuPage());
    if (parent) showMenuPage(parent, menuPage());
    else closeMenu();
  };
  const menuItems = () => MENU[menuPage()];
  const menuMove = (delta: number) => {
    const count = menuItems().length;
    setMenuRow((row) => (row + delta + count) % count);
  };
  /** Step the highlighted menu row's value forward or back, or open the group or dialog it names. */
  const menuChange = (delta: number) => {
    const item = menuItems()[menuRow()];
    if (item === undefined) return;
    if (isMenuPage(item)) {
      if (delta < 0) return;
      showMenuPage(item);
      return;
    }
    switch (item) {
      case "theme":
        setThemeId((id) => cycle(THEMES.map((theme) => theme.id), id, delta));
        break;
      case "font":
        setFont((id) => cycle(FONTS.map((entry) => entry.id), id, delta));
        break;
      case "view":
        setView(cycle(VIEW_MODES, view(), delta));
        break;
      case "details":
        setDetail(cycle(DETAIL_LEVELS, detail(), delta));
        break;
      case "backdrop":
        setBackdropOn((on) => !on);
        break;
      case "iconBox":
        setIconBoxOn((on) => !on);
        break;
      case "statusBar":
        setStatusBarOn((on) => !on);
        break;
      case "clock":
        setClockFormat((format) => cycle(CLOCK_FORMATS, format, delta));
        break;
      case "batteryPercent":
        setBatteryPercentOn((on) => !on);
        break;
      case "categories":
        openCategoryManager();
        break;
      case "fetchArt":
        scrape.start();
        break;
      case "cleanArt":
        openCleanup();
        break;
      case "rescan":
        openRescan();
        break;
      case "updateChannel":
        setUpdateChannel((channel) => cycle(UPDATE_CHANNELS, channel, delta));
        break;
      case "checkUpdates":
        update.check();
        break;
      case "confirm":
        toggleConfirm();
        break;
      case "language":
        setLanguage((id) => cycle(LANGUAGES.map((entry) => entry.id), id, delta));
        break;
      case "diagnostics":
        openDiagnostics();
        break;
    }
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

  /** Add the edited title to "Favorites" or take it out. */
  const toggleFavorite = () => {
    const game = editorGame();
    if (!game) return;
    patchOverride(game.id, { favorite: game.favorite ? undefined : true });
    clampSelection();
  };

  /** Move the edited title to the next or previous category. The tab stays put. */
  const changeGameCategory = (delta: number) => {
    const game = editorGame();
    if (!game) return;
    const choices = allCategories()
      .filter((item) => !item.smart && (!isHidden(item.id) || item.id === game.category))
      .map((item) => item.id);
    const next = cycle(choices, game.category, delta);
    patchOverride(game.id, { category: next === baseOf(game.id)?.category ? undefined : next });
    clampSelection();
  };

  const resetGame = () => {
    const game = editorGame();
    if (!game) return;
    // The favorite mark is not a change to how the title looks; it stays.
    setOverrides(({ [game.id]: removed, ...rest }) =>
      removed?.favorite ? { ...rest, [game.id]: { favorite: true } } : rest,
    );
    clampSelection();
    setEditorNote(t().resetDone);
  };

  // --- Title keyboard ----------------------------------------------------
  const [keyboardOpen, setKeyboardOpen] = createSignal(false);
  const [keyboardText, setKeyboardText] = createSignal("");
  const [keyRow, setKeyRow] = createSignal(1);
  const [keyCol, setKeyCol] = createSignal(0);
  const [shift, setShift] = createSignal(false);
  const [keyPage, setKeyPage] = createSignal<KeyPage>("letters");
  const accentKeys = createMemo(() => accentRows(language()));
  const keyRows = () => {
    const page = keyPage();
    return page === "symbols" ? SYMBOL_ROWS : page === "accents" ? accentKeys() : LETTER_ROWS;
  };

  const [keyboardTarget, setKeyboardTarget] = createSignal<KeyboardTarget>({ kind: "title" });
  const keyboardLimit = () => {
    const kind = keyboardTarget().kind;
    if (kind === "apiKey") return KEY_MAX;
    return kind === "title" || kind === "search" || kind === "library" ? TITLE_MAX : CATEGORY_LABEL_MAX;
  };
  /** The heading above the keyboard: what is being typed. */
  const keyboardHeading = () => t().keyboardFor[keyboardTarget().kind];
  /** A line beside the typed text: how many titles the search term finds so far. */
  const keyboardNote = () => {
    if (keyboardTarget().kind !== "library" || !keyboardText().trim()) return "";
    return fill(t().found, { count: searchTitles(allGames(), keyboardText()).length });
  };

  const openKeyboard = (target: KeyboardTarget, text: string) => {
    setKeyboardTarget(target);
    setKeyboardText(text);
    setKeyRow(1);
    setKeyCol(0);
    setShift(false);
    setKeyPage("letters");
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
  /** Step to the next page: letters, symbols, accents. The highlight keeps its row. */
  const nextKeyPage = () => {
    const from = keyRows()[keyRow()].length;
    setKeyPage((page) => cycle(KEY_PAGES, page, 1));
    // Rows differ in length between pages; a column past the end has no key.
    setKeyCol((col) => mapColumn(from, keyRows()[keyRow()].length, col));
  };
  const commitKeyboard = () => {
    const text = keyboardText().trim();
    const target = keyboardTarget();
    if (target.kind === "title") {
      const game = editorGame();
      if (game) {
        // Typing the original title back removes the override.
        patchOverride(game.id, { title: !text || text === baseOf(game.id)?.title ? undefined : text });
      }
    } else if (target.kind === "library") {
      applySearch(text);
    } else if (target.kind === "search") {
      online.submitTerm(text);
    } else if (target.kind === "apiKey") {
      // The key is asked for by whichever SteamGridDB panel is open.
      if (scrape.open()) scrape.submitKey(text);
      else online.submitKey(text);
    } else if (text) {
      const used = allCategories().some(
        (item) =>
          item.label.toLowerCase() === text.toLowerCase() &&
          (target.kind === "newCategory" || item.id !== target.id),
      );
      if (used) {
        setCatNote(t().nameTaken);
      } else if (target.kind === "newCategory") {
        const id = makeCategoryId(text, new Set([SEARCH_ID, ...allCategories().map((item) => item.id)]));
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
    else if (key.action === "symbols") nextKeyPage();
    else commitKeyboard();
  };

  // --- File picker: box art and backdrop ----------------------------------
  // Two fixed rows, then the PNG files of the folder. For box art the fixed
  // rows are "Default" and "Game icon"; for a backdrop, "Default" and "None".
  const [artOpen, setArtOpen] = createSignal(false);
  const [pickerKind, setPickerKind] = createSignal<PickerKind>("art");
  // Every PNG file of the folder, and whether the picker lists them all or
  // only the ones named after the edited title.
  const [folderFiles, setFolderFiles] = createSignal<string[]>([]);
  const [pickerEvery, setPickerEvery] = createSignal(false);
  const [artRow, setArtRow] = createSignal(0);
  const [artNote, setArtNote] = createSignal("");
  // Whether the host could decode a listed file, by `<kind>:<file>`, learned
  // from loading its preview. The host answers -1 for a file that is not a
  // PNG, is damaged, or is too large.
  const decodes = new Map<string, boolean>();
  const decodeKey = (file: string) => `${pickerKind()}:${file}`;
  // A press of confirm on a file not yet checked: it is used once the check passes.
  let confirmWaiting = false;

  /** The file the edited title uses now, when the user chose one. */
  const chosenFile = (): string | undefined => {
    const game = editorGame();
    if (pickerKind() === "backdrop") return game?.backdrop === NO_BACKDROP ? undefined : game?.backdrop;
    return game?.artAuto ? undefined : game?.art;
  };
  /**
   * The folder's files named after the edited title: its id, its name, or
   * the name it came with. Fetched pictures start with the id (see
   * `assetFile`). The file in use is listed whatever its name.
   */
  const titleFiles = (): string[] => {
    const game = editorGame();
    if (!game) return [];
    const names = [game.title, baseOf(game.id)?.title ?? "", game.id];
    const chosen = chosenFile();
    return folderFiles().filter((file) => file === chosen || fileBelongsTo(file, names));
  };
  const artFiles = createMemo(() => (pickerEvery() ? folderFiles() : titleFiles()));

  const openArtPicker = (kind: PickerKind = "art") => {
    const game = editorGame();
    setPickerKind(kind);
    if (kind === "art") {
      const files = listArt();
      setArtIndex(new Map(files.map((name) => [name.toLowerCase(), name])));
      setFolderFiles(files);
    } else setFolderFiles(listBackdrops());
    // A title with no files of its own starts on the whole folder.
    setPickerEvery(titleFiles().length === 0);
    const current = artFiles().indexOf(chosenFile() ?? "");
    const fixed = kind === "art" ? game?.artIcon : game?.backdrop === NO_BACKDROP;
    setArtRow(fixed ? 1 : current >= 0 ? current + 2 : 0);
    setArtNote("");
    decodes.clear();
    confirmWaiting = false;
    setArtOpen(true);
  };
  const closeArtPicker = () => setArtOpen(false);
  /** Switch between this title's files and the whole folder. The highlight stays on its file. */
  const togglePickerFiles = () => {
    const file = artFiles()[artRow() - 2];
    setPickerEvery((every) => !every);
    const index = file === undefined ? -1 : artFiles().indexOf(file);
    setArtRow((row) => (index >= 0 ? index + 2 : Math.min(row, 1)));
  };

  // The highlighted row's picture, shown beside the picker. It loads once the
  // highlight has rested on a row, so scrolling does not decode every file.
  const [pickerPreview, setPickerPreview] = createSignal<string | undefined>(undefined);
  let previewWait = 0;
  // Texture handle of the backdrop shown, given back when it leaves the screen.
  let previewBackdrop = -1;
  const clearPickerPreview = () => {
    releaseBackdrop(previewBackdrop);
    previewBackdrop = -1;
    setPickerPreview(undefined);
  };
  createEffect(() => {
    const file = artFiles()[artRow() - 2];
    pickerKind();
    clearPickerPreview();
    confirmWaiting = false;
    setArtNote(file !== undefined && decodes.get(decodeKey(file)) === false ? t().cantRead : "");
    previewWait = artOpen() ? PREVIEW_DELAY : 0;
  });
  /** Record whether `file` decodes, and finish a confirm that waited for it. */
  const checkedFile = (file: string, ok: boolean) => {
    decodes.set(decodeKey(file), ok);
    if (!ok) setArtNote(t().cantRead);
    const waiting = confirmWaiting;
    confirmWaiting = false;
    if (waiting && ok) artConfirm();
  };
  const loadPickerPreview = () => {
    const game = editorGame();
    if (!game || !artOpen()) return;
    const row = artRow();
    const file = row >= 2 ? artFiles()[row - 2] : undefined;
    if (pickerKind() === "backdrop") {
      // Row 0 is the title's own picture, row 1 is none.
      if (row === 1) return;
      const handle = native ? acquireBackdrop(game.id, file ?? "") : -1;
      if (handle === TEXTURE_PENDING) {
        previewWait = PREVIEW_RETRY;
        return;
      }
      if (handle >= 0) {
        const key = `backdrop.${game.id}.${file ?? ""}.${handle}`;
        registerTexture(key, handle);
        previewBackdrop = handle;
        setPickerPreview(key);
      }
      // Last: a confirm that waited for this file closes the drawer.
      if (file !== undefined) checkedFile(file, handle >= 0);
      return;
    }
    // Row 0 is a file matched by name, or the title's own icon; row 1 is that icon.
    const name = file ?? (row === 0 ? findArt(game.id, baseOf(game.id)?.title ?? "", game.title) : undefined);
    const handle = name ? artTexture(name) : appIcon(game.id);
    const key = name ? `art.${name}` : `installed.${game.id}`;
    if (handle === TEXTURE_PENDING) {
      previewWait = PREVIEW_RETRY;
      return;
    }
    if (handle >= 0) {
      if (!registeredKeys.has(key)) {
        registeredKeys.add(key);
        registerTexture(key, handle);
      }
      setPickerPreview(key);
    }
    if (file !== undefined) checkedFile(file, handle >= 0);
  };
  const artMove = (dy: number) => {
    const count = artFiles().length + 2;
    setArtRow((row) => (row + dy + count) % count);
  };
  const artConfirm = () => {
    const game = editorGame();
    if (!game) return;
    const row = artRow();
    // A file is used only once the host has decoded it: one it cannot read
    // keeps the drawer open with a note, one not checked yet is checked first.
    const file = artFiles()[row - 2];
    if (file !== undefined && native) {
      const ok = decodes.get(decodeKey(file));
      if (ok === false) {
        setArtNote(t().cantRead);
        return;
      }
      if (ok === undefined) {
        confirmWaiting = true;
        previewWait = 1;
        return;
      }
    }
    if (pickerKind() === "backdrop") {
      // Row 0 goes back to the title's own picture, row 1 draws none.
      const name = row === 0 ? undefined : row === 1 ? NO_BACKDROP : artFiles()[row - 2];
      patchOverride(game.id, { backdrop: name });
      setArtOpen(false);
      return;
    }
    // Row 0 goes back to a file matched by name (or the title's own icon),
    // row 1 always shows the title's own icon.
    const name = row === 0 ? undefined : row === 1 ? USE_ICON : artFiles()[row - 2];
    patchOverride(game.id, { art: name });
    loadIconsAround();
    setArtOpen(false);
  };

  // --- SteamGridDB (title editor -> SteamGridDB) ----------------------------
  const online = createOnlineFlow({
    t,
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

  // --- Fetch artwork for many titles (SELECT menu -> Fetch artwork) --------
  // "All" leaves out the system applications: a search for "Settings" or
  // "Music" finds an unrelated game. Their own category can still be chosen.
  const scrapeScopes = (): ScrapeScope[] => [
    { id: "", label: t().allTitles, titles: allGames().filter((game) => baseOf(game.id)?.category !== SYSTEM_ID) },
    ...allCategories()
      .filter((item) => !item.smart)
      .map((item) => ({ id: item.id, label: item.label, titles: titlesOf(item.id) }))
      .filter((item) => item.titles.length > 0),
  ];
  const update = createUpdateFlow({
    t,
    channel: updateChannel,
    idle: () => modal() === null,
  });

  const scrape = createScrapeFlow({
    t,
    scopes: scrapeScopes,
    // Start on the current category when it is one of the choices.
    startScope: () => Math.max(0, scrapeScopes().findIndex((item) => item.id === categoryId())),
    askKey: () => openKeyboard({ kind: "apiKey" }, ""),
    apply: (titleId, kind, file) => {
      if (kind === "icon") {
        // The new file has to be in the name index before it can be an icon.
        setArtIndex(readArtIndex());
        patchOverride(titleId, { art: file });
        loadIconsAround();
      } else patchOverride(titleId, { backdrop: file });
    },
  });

  // --- Clean up artwork (SELECT menu -> Clean up artwork) --------------------
  // Deletes files from the art and backdrops folders: the ones no title uses,
  // or all of them. Two rows: which files, then "Delete", pressed twice.
  const [cleanOpen, setCleanOpen] = createSignal(false);
  const [cleanRow, setCleanRow] = createSignal(0);
  const [cleanEvery, setCleanEvery] = createSignal(false);
  const [cleanArmed, setCleanArmed] = createSignal(false);
  const [cleanNote, setCleanNote] = createSignal("");
  // The folders as they were read when the panel opened or last deleted.
  const [cleanFolders, setCleanFolders] = createSignal<{ art: string[]; backdrop: string[] }>({
    art: [],
    backdrop: [],
  });
  const readCleanFolders = () => setCleanFolders({ art: listArt(), backdrop: listBackdrops() });
  /** The files the chosen mode deletes, per folder. */
  const cleanTargets = (): { art: string[]; backdrop: string[] } => {
    const folders = cleanFolders();
    if (cleanEvery()) return folders;
    // In use: a file a title's changes name, or an icon matched to a title by its name.
    const art = filesInUse(overrides(), "art");
    for (const game of allGames()) if (game.art) art.add(game.art);
    const backdrop = filesInUse(overrides(), "backdrop");
    return {
      art: folders.art.filter((file) => !art.has(file)),
      backdrop: folders.backdrop.filter((file) => !backdrop.has(file)),
    };
  };
  const cleanCount = () => cleanTargets().art.length + cleanTargets().backdrop.length;

  const openCleanup = () => {
    readCleanFolders();
    setArtIndex(readArtIndex());
    batch(() => {
      setCleanRow(0);
      setCleanEvery(false);
      setCleanArmed(false);
      setCleanNote("");
      setCleanOpen(true);
    });
  };
  const closeCleanup = () => setCleanOpen(false);
  const cleanMove = (dx: number, dy: number) => {
    batch(() => {
      setCleanArmed(false);
      setCleanNote("");
      if (dy !== 0) setCleanRow((row) => (row + dy + 2) % 2);
      else if (cleanRow() === 0 && dx !== 0) setCleanEvery((every) => !every);
    });
  };
  const deleteCleanTargets = () => {
    const targets = cleanTargets();
    let deleted = 0;
    for (const [folder, files] of [["art", targets.art], ["backdrops", targets.backdrop]] as const) {
      for (const file of files) {
        try {
          rmSync(`${folder}/${file}`, { force: true });
          deleted++;
        } catch {
          // The file stays and is counted again when the folders are read.
        }
      }
    }
    // With every file gone, the titles that named one go back to their own pictures.
    if (cleanEvery()) setOverrides(withoutFiles);
    readCleanFolders();
    setArtIndex(readArtIndex());
    loadIconsAround();
    return deleted;
  };
  /** On "Delete", press twice: the first press asks, the second deletes. */
  const cleanConfirm = () => {
    if (cleanRow() === 0) {
      cleanMove(1, 0);
      return;
    }
    if (cleanCount() === 0) {
      setCleanNote(t().cleanNothing);
    } else if (cleanArmed()) {
      const deleted = deleteCleanTargets();
      batch(() => {
        setCleanArmed(false);
        setCleanNote(fill(t().cleanDone, { count: deleted }));
      });
    } else {
      setCleanArmed(true);
    }
  };

  // --- Rescan titles (SELECT menu -> Rescan titles) -------------------------
  // The host reads the title list from a file; a scan of the installed
  // titles runs on the first start (the scan page) and when the user asks for
  // one here. It runs on the host's worker thread; `pumpRescan` follows it
  // once per frame and the drawer shows its progress.
  const [rescanOpen, setRescanOpen] = createSignal(false);
  const [rescanNote, setRescanNote] = createSignal("");
  const [rescanProgress, setRescanProgress] = createSignal<ScanProgress | null>(null);
  const rescanning = () => rescanProgress()?.state === "running";
  const openRescan = () => {
    batch(() => {
      setRescanNote("");
      setRescanProgress(null);
      setRescanOpen(true);
    });
  };
  const closeRescan = () => {
    if (!rescanning()) setRescanOpen(false);
  };
  const startRescan = () => {
    if (rescanning()) return;
    if (!startScan()) {
      setRescanNote(t().scanFailed);
      return;
    }
    batch(() => {
      setRescanNote("");
      setRescanProgress(scanProgress());
    });
  };
  const pumpRescan = () => {
    if (!rescanning()) return;
    const now = scanProgress();
    setRescanProgress(now);
    if (now.state !== "done") return;
    const games = loadCatalog().games;
    // The files the user may have edited by hand are read again too.
    const changes = reloadOverrides();
    batch(() => {
      setCatalogGames(games);
      setOverrides(changes);
      setRescanNote(fill(t().scanDone, { count: games.length, seconds: (now.ms / 1000).toFixed(1) }));
    });
    clampSelection();
  };

  // --- Diagnostics (SELECT menu -> Diagnostics) ------------------------------
  // The startup timing of this launch (diagnostics.ts), as a scrolling list.
  const [diagnosticsOpen, setDiagnosticsOpen] = createSignal(false);
  // Counted without a limit; the screen wraps it to its number of lines.
  const [diagnosticsRow, setDiagnosticsRow] = createSignal(0);
  const openDiagnostics = () => {
    setDiagnosticsRow(0);
    setDiagnosticsOpen(true);
  };

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
    else setCatNote(t().builtinKeepNames);
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
      setCatNote(t().builtinNoDelete);
      return;
    }
    if (catArmed() === item.id) {
      deleteCategory(item.id);
      setCatArmed(null);
      setCatNote(t().deleted);
    } else {
      // While a row is armed the manager shows the "press again" prompt, not this note.
      setCatArmed(item.id);
      setCatNote("");
    }
  };
  /** Show or hide the highlighted category in the tab bar. */
  const catToggleHidden = () => {
    const item = catItem();
    if (!item) return;
    setCatArmed(null);
    if (isHidden(item.id)) {
      setHiddenCategories((list) => list.filter((id) => id !== item.id));
      setCatNote(fill(t().nowShown, { name: item.label }));
      return;
    }
    // A smart category shows no tab while it is empty, so it does not count.
    if (!item.smart && allCategories().filter((other) => !other.smart && !isHidden(other.id)).length <= 1) {
      setCatNote(t().oneVisible);
      return;
    }
    setHiddenCategories((list) => [...list, item.id]);
    setCatNote(fill(t().nowHidden, { name: item.label }));
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
    if (adrPlugin() !== null) return "adrenaline";
    if (keyboardOpen()) return "keyboard";
    if (artOpen()) return "art";
    if (online.open()) return "online";
    if (scrape.open()) return "scrape";
    if (update.open()) return "update";
    if (cleanOpen()) return "clean";
    if (rescanOpen()) return "rescan";
    if (diagnosticsOpen()) return "diagnostics";
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
        else if (editorRow() === 0) toggleFavorite();
        else if (editorRow() === 1) changeGameCategory(dx);
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
      case "scrape":
        scrape.move(dx, dy);
        break;
      case "clean":
        cleanMove(dx, dy);
        break;
      case "diagnostics":
        // Up from the first line stays there; the screen wraps going down.
        if (dy !== 0) setDiagnosticsRow((row) => Math.max(0, row + dy));
        break;
      case "categories":
        if (dy !== 0) catMove(dy);
        break;
    }
  };
  const modalConfirm = () => {
    switch (modal()) {
      case "adrenaline":
        adrenalineConfirm();
        break;
      case "menu":
        menuChange(1);
        break;
      case "editor": {
        const row = editorRow();
        if (row === 0) toggleFavorite();
        else if (row === 1) changeGameCategory(1);
        else if (row === 2) openKeyboard({ kind: "title" }, editorGame()?.title ?? "");
        else if (row === 3) openArtPicker("art");
        else if (row === 4) openArtPicker("backdrop");
        else if (row === 5) online.start();
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
      case "scrape":
        scrape.confirm();
        break;
      case "update":
        update.confirm();
        break;
      case "clean":
        cleanConfirm();
        break;
      case "rescan":
        startRescan();
        break;
      case "categories":
        catConfirm();
        break;
    }
  };
  const modalCancel = () => {
    switch (modal()) {
      case "adrenaline":
        closeAdrenaline();
        break;
      case "keyboard":
        closeKeyboard();
        break;
      case "art":
        closeArtPicker();
        break;
      case "online":
        online.cancel();
        break;
      case "scrape":
        scrape.cancel();
        break;
      case "update":
        update.cancel();
        break;
      case "clean":
        closeCleanup();
        break;
      case "rescan":
        closeRescan();
        break;
      case "diagnostics":
        setDiagnosticsOpen(false);
        break;
      case "editor":
        closeEditor();
        break;
      case "categories":
        closeCategoryManager();
        break;
      case "menu":
        menuBack();
        break;
    }
  };

  timed("icons", loadIconsAround);

  update.checkAtStart();

  return {
    games,
    categories,
    categoryId,
    categoryCount,
    changeCategory,
    native,
    language,
    t,
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
    statusBarOn,
    clockFormat,
    batteryPercentOn,
    status,
    iconBox,
    backdrop,
    backdropUnder,
    frame,
    select,
    page,
    launchSelected,
    adrPlugin,
    adrFailed,
    searching,
    searchTerm,
    openSearch,
    closeSearch,
    keyboardNote,
    keyboardHeading,
    menuOpen,
    menuRow,
    menuPage,
    menuItems,
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
    keyPage,
    keyRows,
    toggleShift,
    nextKeyPage,
    backspace,
    commitKeyboard,
    artFiles,
    pickerEvery,
    togglePickerFiles,
    pickerPreview,
    artRow,
    artNote,
    pickerKind,
    online,
    scrape,
    update,
    updateChannel,
    cleanRow,
    cleanEvery,
    cleanArmed,
    cleanNote,
    cleanCount,
    rescanNote,
    rescanProgress,
    diagnosticsRow,
    titleCount: () => catalogGames().length,
    allCategories,
    hasTitles: () => catalogGames().length > 0,
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
