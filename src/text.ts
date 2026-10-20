// Text classes for each role the launcher draws text in.
//
// The engine bakes one family into its sans slots: Figtree, set by the
// shelf:build script. Class strings are compiled at build time, so each role
// spells out one full literal. Colors are not part of these classes:
// components set `textColor` from the theme.
//
// Until 2026-10-20 the SELECT menu offered a second font, Hack, in the
// engine's monospace slots. Those slots hold one weight at 12, 14 and 16 px
// only, so a second font there had no bold and no large sizes; the setting
// was removed. A saved `"font": "mono"` is ignored.

/** The one font. Kept as a type so `fontSlot` callers read the same. */
export type FontId = "sans";

export const DEFAULT_FONT: FontId = "sans";

/** Where a piece of text is used. Each role has one size and weight. */
export type TextRole = "display" | "heading" | "title" | "body" | "bodyBold" | "small" | "smallBold" | "label";

const CLASSES: Record<TextRole, string> = {
  display: "text-2xl font-bold",
  heading: "text-xl font-bold",
  title: "text-base font-bold",
  body: "text-sm",
  bodyBold: "text-sm font-bold",
  small: "text-xs",
  smallBold: "text-xs font-bold",
  label: "text-xs font-bold tracking-wide",
};

// Baked font slot per role, for measuring text before it is drawn. Slots 0-6
// are the regular sizes (12, 14, 16, 18, 20, 24, 36), 7-13 their bold pairs.
const SLOTS: Record<TextRole, number> = {
  display: 12,
  heading: 11,
  title: 9,
  body: 1,
  bodyBold: 8,
  small: 0,
  smallBold: 7,
  label: 7,
};

/** Text class per role. */
export function textClasses(_font: FontId = DEFAULT_FONT): Record<TextRole, string> {
  return CLASSES;
}

export function fontSlot(_font: FontId, role: TextRole): number {
  return SLOTS[role];
}
