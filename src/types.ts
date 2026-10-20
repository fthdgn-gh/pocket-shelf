import type { Platform } from "./categories.ts";

export type ViewMode = "carousel" | "grid" | "list" | "cascade" | "home";
/** Built-in ids are "games", "system" and "homebrew"; smart and custom categories have their own. */
export type CategoryId = string;
/** How much text each title shows: none, its title, or title and title id. */
export type DetailLevel = "basic" | "normal" | "detailed";
export type ConfirmMode = "circle" | "cross";

/** One installed title as the launcher shows it. */
export interface Game {
  title: string;
  /** Title id, for example PCSA00069. Also the key for `appLaunch` and `appIcon`. */
  id: string;
  genre: string;
  category: CategoryId;
  /** True when the user marked the title as a favorite. */
  favorite?: boolean;
  /** Custom box art: a file name in the launcher's art folder. */
  art?: string;
  /** True when `art` was found by name, not chosen by the user. */
  artAuto?: boolean;
  /** True when the user chose the title's own icon over a matching art file. */
  artIcon?: boolean;
  /** The user's backdrop: a file name in the backdrops folder, or "none". */
  backdrop?: string;
  /** Index of the tint used behind the title's art and for the screen's ambient color. */
  tint: number;
  /** A PSP or PS1 title with no bubble, started through Adrenaline. */
  adrenaline?: boolean;
  /** Where the title lives, as the host reports it (vita, psm, psp, psx or retro). */
  platform?: Platform;
  /**
   * A RetroArch game: the console's database name from RetroArch's playlist
   * (`Nintendo - Game Boy`). Its `id` is the host's (`ra` and seven digits).
   */
  console?: string;
}

/** How the status bar's clock is written: the system setting, or a fixed 12- or 24-hour clock. */
export type ClockFormat = "system" | "12" | "24";
