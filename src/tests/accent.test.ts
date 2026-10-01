// Unit tests for the icon box colors and the Dynamic theme's palette. Run with `bun run shelf:test`.

import { describe, expect, test } from "bun:test";
import { ACCENT_HAS_ALPHA, boxColors, hslHex, hueSaturation } from "../accent.ts";
import { dynamicTheme } from "../themes.ts";

describe("hueSaturation", () => {
  test("reads the primary hues", () => {
    expect(hueSaturation(0xff0000)).toEqual([0, 1]);
    expect(hueSaturation(0x00ff00)).toEqual([120, 1]);
    expect(hueSaturation(0x0000ff)).toEqual([240, 1]);
  });

  test("a red that leans toward blue wraps to the top of the circle", () => {
    const [hue] = hueSaturation(0xff0040);
    expect(hue).toBeGreaterThan(330);
    expect(hue).toBeLessThan(360);
  });

  test("gray has no saturation", () => {
    expect(hueSaturation(0x808080)).toEqual([0, 0]);
    expect(hueSaturation(0x000000)).toEqual([0, 0]);
  });
});

describe("hslHex", () => {
  test("round-trips known colors", () => {
    expect(hslHex(0, 1, 0.5)).toBe("#ff0000");
    expect(hslHex(120, 1, 0.5)).toBe("#00ff00");
    expect(hslHex(240, 1, 0.5)).toBe("#0000ff");
    expect(hslHex(0, 0, 0.5)).toBe("#808080");
    expect(hslHex(200, 0, 0)).toBe("#000000");
  });
});

describe("boxColors", () => {
  test("an icon with no transparent parts gets no box", () => {
    expect(boxColors(0x2060e0)).toBeUndefined();
    expect(boxColors(-1)).toBeUndefined();
  });

  test("a see-through icon gets a darker box in its own hue", () => {
    const colors = boxColors(ACCENT_HAS_ALPHA | 0x2060e0);
    expect(colors).toBeDefined();
    if (!colors) return;
    const top = Number.parseInt(colors.from.slice(1), 16);
    const bottom = Number.parseInt(colors.to.slice(1), 16);
    const [accentHue] = hueSaturation(0x2060e0);
    expect(Math.abs(hueSaturation(top)[0] - accentHue)).toBeLessThan(3);
    expect(Math.abs(hueSaturation(bottom)[0] - accentHue)).toBeLessThan(3);
    // Blue stays the strongest channel, and the bottom is darker than the top.
    expect(top & 255).toBeGreaterThan((top >> 16) & 255);
    expect(bottom & 255).toBeLessThan(top & 255);
  });

  test("a neon accent is toned down and a gray one stays gray", () => {
    const neon = boxColors(ACCENT_HAS_ALPHA | 0x00ff00);
    expect(neon).toEqual({ from: hslHex(120, 0.6, 0.36), to: hslHex(120, 0.6, 0.17) });
    const gray = boxColors(ACCENT_HAS_ALPHA | 0x969696);
    expect(gray).toEqual({ from: "#5c5c5c", to: "#2b2b2b" });
  });
});

describe("dynamicTheme", () => {
  const HEX = /^#[0-9a-f]{6}$/;
  const lightness = (hex: string) => {
    const rgb = Number.parseInt(hex.slice(1), 16);
    const parts = [(rgb >> 16) & 255, (rgb >> 8) & 255, rgb & 255];
    return (Math.max(...parts) + Math.min(...parts)) / 510;
  };

  test("every color is six hex digits, so an alpha pair can be appended", () => {
    for (const rgb of [0xff0000, 0x2060e0, 0x808080, 0x000000, 0xffffff, 0xf59e0b]) {
      const theme = dynamicTheme(rgb);
      for (const key of ["bgTop", "bgBottom", "text", "dim", "faint", "accent", "panel", "line", "tile"] as const) {
        expect(theme[key]).toMatch(HEX);
      }
    }
  });

  test("the screen is dark and the accent bright, in the title's hue", () => {
    const theme = dynamicTheme(0x2060e0);
    const [hue] = hueSaturation(0x2060e0);
    expect(lightness(theme.bgTop)).toBeLessThan(0.2);
    expect(lightness(theme.bgBottom)).toBeLessThan(lightness(theme.bgTop));
    expect(lightness(theme.accent)).toBeGreaterThan(0.55);
    for (const hex of [theme.bgTop, theme.accent, theme.panel]) {
      expect(Math.abs(hueSaturation(Number.parseInt(hex.slice(1), 16))[0] - hue)).toBeLessThan(4);
    }
  });

  test("a title without color gets a gray palette", () => {
    const theme = dynamicTheme(0x969696);
    for (const hex of [theme.bgTop, theme.bgBottom, theme.panel, theme.dim]) {
      expect(hueSaturation(Number.parseInt(hex.slice(1), 16))[1]).toBe(0);
    }
    expect(theme.accent).toBe("#e5e7eb");
  });
});
