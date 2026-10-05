/**
 * The SELECT menu's pages. A row named after a page opens that page: the main
 * page holds Appearance, Library and Updates, and Appearance holds Status bar. Language is second to
 * last on the main page, two presses up from the first row, so it can be
 * found in a language the user cannot read.
 */
export type MenuPage = "main" | "appearance" | "status" | "library" | "updates";
export type MenuItem =
  | MenuPage
  | "theme"
  | "font"
  | "view"
  | "details"
  | "backdrop"
  | "iconBox"
  | "statusBar"
  | "clock"
  | "batteryPercent"
  | "categories"
  | "fetchArt"
  | "cleanArt"
  | "rescan"
  | "updateChannel"
  | "checkUpdates"
  | "confirm"
  | "language"
  | "diagnostics";
export const MENU: Record<MenuPage, readonly MenuItem[]> = {
  main: ["appearance", "library", "updates", "confirm", "language", "diagnostics"],
  appearance: ["theme", "font", "view", "details", "backdrop", "iconBox", "status"],
  status: ["statusBar", "clock", "batteryPercent"],
  library: ["categories", "fetchArt", "cleanArt", "rescan"],
  updates: ["updateChannel", "checkUpdates"],
};
export const isMenuPage = (item: MenuItem): item is MenuPage => item in MENU;

/** The page that holds the row opening `page`; the main page has none. */
export function parentPage(page: MenuPage): MenuPage | undefined {
  return (Object.keys(MENU) as MenuPage[]).find((key) => MENU[key].includes(page));
}

/** The pages to open, from the main page down, to reach the page holding `item`. */
export function menuPath(item: MenuItem): MenuPage[] {
  const holder = (Object.keys(MENU) as MenuPage[]).find((key) => MENU[key].includes(item));
  if (!holder) return [];
  const path: MenuPage[] = [holder];
  for (let page = parentPage(holder); page; page = parentPage(page)) path.unshift(page);
  return path;
}
