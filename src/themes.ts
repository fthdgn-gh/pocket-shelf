// Launcher themes.
//
// A theme is a palette. Layout lives in class literals next to the component
// that uses them, and colors are applied through `style` props (`bgColor`,
// `textColor`, `gradFrom`, ...), which take values at run time. Add a theme by
// copying one block, giving it a new id, and adding the id to `ThemeId`.
//
// Colors are `#rrggbb`. `accent` and `shelf` stay six digits because
// components append an alpha pair to them (see `alpha`).

export type ThemeId = "midnight" | "aurora" | "sakura" | "ember" | "daylight";

export interface Theme {
  id: ThemeId;
  name: string;
  /** Screen background, top and bottom of a vertical gradient. */
  bgTop: string;
  bgBottom: string;
  /** Primary text, secondary text, and the weakest text (hints, separators). */
  text: string;
  dim: string;
  faint: string;
  /** Selected tile frame, active tab, highlighted rows and values. */
  accent: string;
  /** Drawers and sheets. */
  panel: string;
  /** Hairlines and badge outlines. */
  line: string;
  /** Tile background behind an icon, and keys of the keyboard. */
  tile: string;
  /** Dimmer over the screen while a panel is open. */
  scrim: string;
}

export const DEFAULT_THEME: ThemeId = "midnight";

export const THEMES: readonly Theme[] = [
  {
    id: "midnight",
    name: "Midnight",
    bgTop: "#101a36",
    bgBottom: "#02040c",
    text: "#ffffff",
    dim: "#9aa8c0",
    faint: "#55627a",
    accent: "#22d3ee",
    panel: "#0b1226",
    line: "#2a3654",
    tile: "#1a2440",
    scrim: "#00000099",
  },
  {
    id: "aurora",
    name: "Aurora",
    bgTop: "#0a2e24",
    bgBottom: "#020b08",
    text: "#ecfdf5",
    dim: "#8fb8a8",
    faint: "#4a7163",
    accent: "#34d399",
    panel: "#071c16",
    line: "#1d4638",
    tile: "#12352b",
    scrim: "#00000099",
  },
  {
    id: "sakura",
    name: "Sakura",
    bgTop: "#33122b",
    bgBottom: "#0d040b",
    text: "#fff1f7",
    dim: "#cfa3bd",
    faint: "#7d5069",
    accent: "#f472b6",
    panel: "#210c1c",
    line: "#4d2342",
    tile: "#3a1731",
    scrim: "#00000099",
  },
  {
    id: "ember",
    name: "Ember",
    bgTop: "#33180a",
    bgBottom: "#0c0502",
    text: "#fff7ed",
    dim: "#c2a48f",
    faint: "#7a5b45",
    accent: "#fb923c",
    panel: "#200f06",
    line: "#4d2c18",
    tile: "#3b1e0e",
    scrim: "#00000099",
  },
  {
    id: "daylight",
    name: "Daylight",
    bgTop: "#f8fafc",
    bgBottom: "#d5deea",
    text: "#0f172a",
    dim: "#475569",
    faint: "#94a3b8",
    accent: "#4f46e5",
    panel: "#ffffff",
    line: "#cbd5e1",
    tile: "#e2e8f0",
    scrim: "#0f172a80",
  },
];

export function themeById(id: ThemeId): Theme {
  return THEMES.find((theme) => theme.id === id) ?? THEMES[0];
}

/** A six-digit color with an alpha pair appended, for example `alpha("#22d3ee", "33")`. */
export function alpha(color: string, pair: string): string {
  return `${color}${pair}`;
}
