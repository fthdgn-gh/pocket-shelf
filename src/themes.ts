// Launcher themes.
//
// A theme is a palette. Layout lives in class literals next to the component
// that uses them, and colors are applied through `style` props (`bgColor`,
// `textColor`, `gradFrom`, ...), which take values at run time. Add a theme by
// copying one block, giving it a new id, and adding the id to `ThemeId`.
//
// "Dynamic" is the exception: it has no fixed colors. `dynamicTheme` builds
// its palette from the selected title's color, and its entry in `THEMES` only
// gives it a place in the menu and the colors used before a title is selected.
//
// Colors are `#rrggbb`. `accent` and `shelf` stay six digits because
// components append an alpha pair to them (see `alpha`).

import { hslHex, hueSaturation } from "./accent.ts";

export type ThemeId = "midnight" | "aurora" | "sakura" | "ember" | "daylight" | "dynamic";

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
  {
    id: "dynamic",
    name: "Dynamic",
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
];

/**
 * The Dynamic theme's palette for a title's color (`0xRRGGBB`): a dark screen
 * in the color's hue and a bright accent of the same hue. A color with almost
 * no saturation gives a neutral gray palette.
 */
export function dynamicTheme(rgb: number): Theme {
  const [hue, saturation] = hueSaturation(rgb);
  const colored = saturation >= 0.08;
  // The surfaces keep part of the color's saturation; the accent is always vivid.
  const surface = colored ? Math.min(saturation, 0.55) : 0;
  const hint = colored ? 0.18 : 0;
  return {
    id: "dynamic",
    name: "Dynamic",
    bgTop: hslHex(hue, surface * 0.9, 0.17),
    bgBottom: hslHex(hue, surface, 0.04),
    text: "#ffffff",
    dim: hslHex(hue, hint, 0.72),
    faint: hslHex(hue, hint, 0.42),
    accent: colored ? hslHex(hue, Math.max(saturation, 0.8), 0.62) : "#e5e7eb",
    panel: hslHex(hue, surface * 0.8, 0.09),
    line: hslHex(hue, surface * 0.6, 0.25),
    tile: hslHex(hue, surface * 0.7, 0.19),
    scrim: "#00000099",
  };
}

export function themeById(id: ThemeId): Theme {
  return THEMES.find((theme) => theme.id === id) ?? THEMES[0];
}

/** A six-digit color with an alpha pair appended, for example `alpha("#22d3ee", "33")`. */
export function alpha(color: string, pair: string): string {
  return `${color}${pair}`;
}
