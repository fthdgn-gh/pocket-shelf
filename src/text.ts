// Fonts the launcher can switch between at run time, and the text classes for
// each one.
//
// The engine bakes two families: the sans slots and the monospace slots
// (`font-mono`). Space Grotesk fills the first and Hack the second; both are
// baked into the app (see the shelf:build script). Class strings are compiled
// at build time, so each role spells out one full literal per font. Colors are
// not part of these classes: components set `textColor` from the theme.

export type FontId = "sans" | "mono";

export const FONTS: readonly { id: FontId; name: string }[] = [
  { id: "sans", name: "Space Grotesk" },
  { id: "mono", name: "Hack" },
];

export const DEFAULT_FONT: FontId = "sans";

/** Where a piece of text is used. Each role has one size and weight per font. */
export type TextRole = "heading" | "title" | "body" | "bodyBold" | "small" | "smallBold" | "label";

// Monospace is baked at 12, 14 and 16 px in one weight, so its larger and
// bolder roles fall back to the nearest size it has.
const CLASSES: Record<FontId, Record<TextRole, string>> = {
  sans: {
    heading: "text-xl font-bold",
    title: "text-base font-bold",
    body: "text-sm",
    bodyBold: "text-sm font-bold",
    small: "text-xs",
    smallBold: "text-xs font-bold",
    label: "text-xs font-bold tracking-wide",
  },
  mono: {
    heading: "font-mono text-base",
    title: "font-mono text-base",
    body: "font-mono text-sm",
    bodyBold: "font-mono text-sm",
    small: "font-mono text-xs",
    smallBold: "font-mono text-xs",
    label: "font-mono text-xs",
  },
};

// Baked font slot per role, for measuring text before it is drawn. Slots 0-6
// are the regular sizes (12, 14, 16, 18, 20, 24, 36), 7-13 their bold pairs,
// and 16-18 monospace at 12, 14 and 16.
const SLOTS: Record<FontId, Record<TextRole, number>> = {
  sans: { heading: 11, title: 9, body: 1, bodyBold: 8, small: 0, smallBold: 7, label: 7 },
  mono: { heading: 18, title: 18, body: 17, bodyBold: 17, small: 16, smallBold: 16, label: 16 },
};

/** Text class per role for the chosen font. */
export function textClasses(font: FontId): Record<TextRole, string> {
  return CLASSES[font];
}

export function fontSlot(font: FontId, role: TextRole): number {
  return SLOTS[font][role];
}
