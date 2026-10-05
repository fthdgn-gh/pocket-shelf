import { readFileSync, writeFileSync } from "@pocketjs/framework/fs";
import { LANGUAGES, type Language } from "./i18n.ts";
import { FONTS, type FontId } from "./text.ts";
import { THEMES, type ThemeId } from "./themes.ts";
import { DETAIL_LEVELS, VIEW_MODES } from "./navigation.ts";
import type { ClockFormat, ConfirmMode, DetailLevel, ViewMode } from "./types.ts";
import { UPDATE_CHANNELS, type UpdateChannel } from "./updates.ts";

export interface Settings {
  language: Language;
  theme: ThemeId;
  font: FontId;
  view: ViewMode;
  detail: DetailLevel;
  confirm: ConfirmMode;
  /** Whether the selected title's picture is drawn behind the screen. */
  backdrop: boolean;
  /** Whether an icon with transparent parts gets a box behind it. */
  iconBox: boolean;
  /** Whether the status bar (time, Wi-Fi, Bluetooth, battery) is shown. */
  statusBar: boolean;
  clock: ClockFormat;
  /** Whether the battery shows its charge as a number. */
  batteryPercent: boolean;
  /** Which releases the app checks GitHub for (src/updates.ts). */
  updates: UpdateChannel;
  /** Category and title that were selected when the settings were last saved. */
  category?: string;
  title?: string;
}

export const CLOCK_FORMATS: readonly ClockFormat[] = ["system", "24", "12"];

// Relative to the data folder, ux0:/data/PocketShelf/.
const SETTINGS_FILE = "settings.json";

/**
 * Saved settings, or null when nothing usable is stored. Values are checked
 * one by one, so a stale or hand-edited file degrades to defaults. The fs
 * namespace is missing on hosts built without `data-fs`; that also returns null.
 */
export function loadSettings(): Partial<Settings> | null {
  try {
    const raw = JSON.parse(readFileSync(SETTINGS_FILE, "utf8")) as Partial<Settings>;
    const result: Partial<Settings> = {};
    if (LANGUAGES.some((item) => item.id === raw.language)) result.language = raw.language;
    if (THEMES.some((theme) => theme.id === raw.theme)) result.theme = raw.theme;
    if (FONTS.some((item) => item.id === raw.font)) result.font = raw.font;
    if (VIEW_MODES.includes(raw.view as ViewMode)) result.view = raw.view;
    if (DETAIL_LEVELS.includes(raw.detail as DetailLevel)) result.detail = raw.detail;
    if (raw.confirm === "circle" || raw.confirm === "cross") result.confirm = raw.confirm;
    if (typeof raw.backdrop === "boolean") result.backdrop = raw.backdrop;
    if (typeof raw.iconBox === "boolean") result.iconBox = raw.iconBox;
    if (typeof raw.statusBar === "boolean") result.statusBar = raw.statusBar;
    if (CLOCK_FORMATS.includes(raw.clock as ClockFormat)) result.clock = raw.clock;
    if (typeof raw.batteryPercent === "boolean") result.batteryPercent = raw.batteryPercent;
    if (UPDATE_CHANNELS.includes(raw.updates as UpdateChannel)) result.updates = raw.updates;
    if (typeof raw.category === "string") result.category = raw.category;
    if (typeof raw.title === "string") result.title = raw.title;
    return result;
  } catch {
    return null;
  }
}

export function saveSettings(settings: Settings): void {
  try {
    writeFileSync(SETTINGS_FILE, JSON.stringify(settings));
  } catch (error) {
    console.log(`Settings not saved: ${error}`);
  }
}
