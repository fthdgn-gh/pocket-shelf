// Fonts the launcher can switch between at run time.
//
// The engine bakes two families: the sans slots and the monospace slots
// (`font-mono`). Space Grotesk fills the first and Hack the second; both are
// baked into the app (see the shelf:build script). Switching swaps each text class
// for its monospace twin. Both spellings are full literals, because class
// strings are compiled at build time.

import type { Theme } from "./themes.ts";

export type FontId = "sans" | "mono";

export const FONTS: readonly { id: FontId; name: string }[] = [
  { id: "sans", name: "Space Grotesk" },
  { id: "mono", name: "Hack" },
];

export const DEFAULT_FONT: FontId = "sans";

/** Baked font slot for a size and weight. Monospace has no bold slot. */
export function fontSlot(font: FontId, px: 12 | 14, bold: boolean): number {
  if (font === "mono") return px === 12 ? 16 : 17;
  return (bold ? 7 : 0) + (px === 12 ? 0 : 1);
}

const MONO_CLASS: Record<string, string> = {
  "text-base font-bold text-[#ffffff]":
    "font-mono text-base font-bold text-[#ffffff]",
  "text-base font-bold text-[#ecfdf5]":
    "font-mono text-base font-bold text-[#ecfdf5]",
  "text-base font-bold text-[#fff1f7]":
    "font-mono text-base font-bold text-[#fff1f7]",
  "text-base font-bold text-[#fff7ed]":
    "font-mono text-base font-bold text-[#fff7ed]",
  "text-base font-bold text-[#0f172a]":
    "font-mono text-base font-bold text-[#0f172a]",
  "text-xs text-[#64748b]":
    "font-mono text-xs text-[#64748b]",
  "text-xs text-[#5f8a7a]":
    "font-mono text-xs text-[#5f8a7a]",
  "text-xs text-[#9c6f8a]":
    "font-mono text-xs text-[#9c6f8a]",
  "text-xs text-[#8c7463]":
    "font-mono text-xs text-[#8c7463]",
  "text-xs text-[#94a3b8]":
    "font-mono text-xs text-[#94a3b8]",
  "text-xs text-[#8fb8a8]":
    "font-mono text-xs text-[#8fb8a8]",
  "text-xs text-[#cfa3bd]":
    "font-mono text-xs text-[#cfa3bd]",
  "text-xs text-[#c2a48f]":
    "font-mono text-xs text-[#c2a48f]",
  "text-xs text-[#475569]":
    "font-mono text-xs text-[#475569]",
  "text-xs font-bold text-[#22d3ee]":
    "font-mono text-xs font-bold text-[#22d3ee]",
  "text-xs font-bold text-[#34d399]":
    "font-mono text-xs font-bold text-[#34d399]",
  "text-xs font-bold text-[#f472b6]":
    "font-mono text-xs font-bold text-[#f472b6]",
  "text-xs font-bold text-[#fb923c]":
    "font-mono text-xs font-bold text-[#fb923c]",
  "text-xs font-bold text-[#4f46e5]":
    "font-mono text-xs font-bold text-[#4f46e5]",
  "text-xs font-bold text-[#ffffff]":
    "font-mono text-xs font-bold text-[#ffffff]",
  "text-xs font-bold text-[#ecfdf5]":
    "font-mono text-xs font-bold text-[#ecfdf5]",
  "text-xs font-bold text-[#fff1f7]":
    "font-mono text-xs font-bold text-[#fff1f7]",
  "text-xs font-bold text-[#fff7ed]":
    "font-mono text-xs font-bold text-[#fff7ed]",
  "text-xs font-bold text-[#0f172a]":
    "font-mono text-xs font-bold text-[#0f172a]",
  "mt-1 text-xs font-bold text-[#ffffff]":
    "font-mono mt-1 text-xs font-bold text-[#ffffff]",
  "mt-1 text-xs font-bold text-[#ecfdf5]":
    "font-mono mt-1 text-xs font-bold text-[#ecfdf5]",
  "mt-1 text-xs font-bold text-[#fff1f7]":
    "font-mono mt-1 text-xs font-bold text-[#fff1f7]",
  "mt-1 text-xs font-bold text-[#fff7ed]":
    "font-mono mt-1 text-xs font-bold text-[#fff7ed]",
  "mt-1 text-xs font-bold text-[#0f172a]":
    "font-mono mt-1 text-xs font-bold text-[#0f172a]",
  "grow text-sm font-bold text-[#ffffff]":
    "font-mono grow text-sm font-bold text-[#ffffff]",
  "grow text-sm font-bold text-[#ecfdf5]":
    "font-mono grow text-sm font-bold text-[#ecfdf5]",
  "grow text-sm font-bold text-[#fff1f7]":
    "font-mono grow text-sm font-bold text-[#fff1f7]",
  "grow text-sm font-bold text-[#fff7ed]":
    "font-mono grow text-sm font-bold text-[#fff7ed]",
  "grow text-sm font-bold text-[#0f172a]":
    "font-mono grow text-sm font-bold text-[#0f172a]",
  "text-sm text-[#94a3b8]":
    "font-mono text-sm text-[#94a3b8]",
  "text-sm text-[#8fb8a8]":
    "font-mono text-sm text-[#8fb8a8]",
  "text-sm text-[#cfa3bd]":
    "font-mono text-sm text-[#cfa3bd]",
  "text-sm text-[#c2a48f]":
    "font-mono text-sm text-[#c2a48f]",
  "text-sm text-[#475569]":
    "font-mono text-sm text-[#475569]",
  "text-sm font-bold text-[#ffffff]":
    "font-mono text-sm font-bold text-[#ffffff]",
  "text-sm font-bold text-[#ecfdf5]":
    "font-mono text-sm font-bold text-[#ecfdf5]",
  "text-sm font-bold text-[#fff1f7]":
    "font-mono text-sm font-bold text-[#fff1f7]",
  "text-sm font-bold text-[#fff7ed]":
    "font-mono text-sm font-bold text-[#fff7ed]",
  "text-sm font-bold text-[#0f172a]":
    "font-mono text-sm font-bold text-[#0f172a]",
  "text-sm font-bold text-[#22d3ee]":
    "font-mono text-sm font-bold text-[#22d3ee]",
  "text-sm font-bold text-[#34d399]":
    "font-mono text-sm font-bold text-[#34d399]",
  "text-sm font-bold text-[#f472b6]":
    "font-mono text-sm font-bold text-[#f472b6]",
  "text-sm font-bold text-[#fb923c]":
    "font-mono text-sm font-bold text-[#fb923c]",
  "text-sm font-bold text-[#4f46e5]":
    "font-mono text-sm font-bold text-[#4f46e5]",
};

const TEXT_ROLES = [
  "title",
  "crumbSep",
  "crumb",
  "pillText",
  "tabText",
  "tabTextActive",
  "cardTitle",
  "cardMeta",
  "cardId",
  "tileTitle",
  "rowTitle",
  "footerAccent",
  "footerDim",
  "empty",
  "menuTitle",
  "menuLabel",
  "menuValue",
  "menuHint",
] as const;

/** The theme with its text classes in the chosen font. */
export function withFont(theme: Theme, font: FontId): Theme {
  if (font === "sans") return theme;
  const result: Theme = { ...theme };
  for (const role of TEXT_ROLES) result[role] = MONO_CLASS[theme[role]] ?? theme[role];
  return result;
}
