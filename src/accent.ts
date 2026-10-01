// The box drawn behind an icon that has transparent parts. The Vita host finds
// the icon's strongest color (hosts/vita/src/accent.rs); this module turns it
// into the box's two gradient colors: the same hue, darkened so the icon's own
// shapes stand out against it.

/** Bit set in the host's packed accent when the icon has transparent parts. */
export const ACCENT_HAS_ALPHA = 1 << 24;

export interface BoxColors {
  /** Top and bottom of the box's vertical gradient, as `#rrggbb`. */
  from: string;
  to: string;
}

// The box keeps the accent's hue. Its saturation is capped and its lightness
// fixed, so a pale or a neon icon both get a box of the same weight.
const SATURATION_MAX = 0.6;
const LIGHTNESS_TOP = 0.36;
const LIGHTNESS_BOTTOM = 0.17;

/** Hue (0-360) and saturation (0-1) of an `0xRRGGBB` color. */
export function hueSaturation(rgb: number): [hue: number, saturation: number] {
  const r = ((rgb >> 16) & 255) / 255;
  const g = ((rgb >> 8) & 255) / 255;
  const b = (rgb & 255) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const spread = max - min;
  if (spread === 0) return [0, 0];
  const lightness = (max + min) / 2;
  const saturation = spread / (1 - Math.abs(2 * lightness - 1));
  let sixth: number;
  if (max === r) sixth = (((g - b) / spread) % 6 + 6) % 6;
  else if (max === g) sixth = (b - r) / spread + 2;
  else sixth = (r - g) / spread + 4;
  return [sixth * 60, Math.min(1, saturation)];
}

/** `#rrggbb` for a hue (0-360), saturation and lightness (0-1). */
export function hslHex(hue: number, saturation: number, lightness: number): string {
  const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
  const sixth = hue / 60;
  const second = chroma * (1 - Math.abs((sixth % 2) - 1));
  const [r, g, b] =
    sixth < 1
      ? [chroma, second, 0]
      : sixth < 2
        ? [second, chroma, 0]
        : sixth < 3
          ? [0, chroma, second]
          : sixth < 4
            ? [0, second, chroma]
            : sixth < 5
              ? [second, 0, chroma]
              : [chroma, 0, second];
  const lift = lightness - chroma / 2;
  const channel = (value: number) =>
    Math.round((value + lift) * 255)
      .toString(16)
      .padStart(2, "0");
  return `#${channel(r)}${channel(g)}${channel(b)}`;
}

/**
 * The box for an icon, from the host's packed accent (`ACCENT_HAS_ALPHA |
 * 0xRRGGBB`, or -1 for none). An icon with no transparent parts covers its
 * whole tile, so it gets no box.
 */
export function boxColors(packed: number): BoxColors | undefined {
  if (packed < 0 || (packed & ACCENT_HAS_ALPHA) === 0) return undefined;
  const [hue, saturation] = hueSaturation(packed & 0xffffff);
  const kept = Math.min(saturation, SATURATION_MAX);
  return { from: hslHex(hue, kept, LIGHTNESS_TOP), to: hslHex(hue, kept, LIGHTNESS_BOTTOM) };
}
