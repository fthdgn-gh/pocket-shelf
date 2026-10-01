// Unit tests for the icon box colors. Run with `bun run shelf:test`.

import { describe, expect, test } from "bun:test";
import { ACCENT_HAS_ALPHA, boxColors, hslHex, hueSaturation } from "../accent.ts";

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
