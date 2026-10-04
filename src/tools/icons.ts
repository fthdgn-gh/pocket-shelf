// Writes Pocket Shelf's button and symbol icons to src/icons/ as SVG.
//
//   bun run shelf:icons
//
// The build bakes each SVG into a texture (framework/compiler/bake-svg.ts).
// That baker reads filled <rect>, <circle> and <path> elements only: no
// strokes, no text, no arcs and no group transforms. So every icon here is
// built from those three, and letters are converted to outlines from Space
// Grotesk Bold. Texture sides are powers of two; an icon narrower than its
// texture sits at the left edge and the UI clips the rest (components/icons.tsx).
//
// The engine cannot recolor a texture, so the icons carry their own colors:
// a neutral dark button face that reads on both the dark themes and Daylight.
// The SVG files are committed; a build does not run this script.

import opentype from "opentype.js";
import { mkdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SRC = resolve(fileURLToPath(new URL("..", import.meta.url)));
const OUT = join(SRC, "icons");

const font = opentype.parse(readFileSync(join(SRC, "fonts/SpaceGrotesk-Bold.ttf")).buffer as ArrayBuffer);

/** Button face. */
const FACE = "#3d4350";
/** Letters and lit D-pad arms. */
const INK = "#ffffff";
/** Arrows and the plus sign, which sit on a panel without a button face. */
const SOFT = "#8b95a7";
// The four face-button symbols, in the colors printed on the buttons.
const TRIANGLE = "#5eead4";
const CIRCLE = "#fb7185";
const CROSS = "#93c5fd";
const SQUARE = "#f0abfc";

const n = (value: number) => Number(value.toFixed(2));
const rect = (x: number, y: number, width: number, height: number, fill: string) =>
  `<rect x="${n(x)}" y="${n(y)}" width="${n(width)}" height="${n(height)}" fill="${fill}"/>`;
const circle = (cx: number, cy: number, r: number, fill: string) =>
  `<circle cx="${n(cx)}" cy="${n(cy)}" r="${n(r)}" fill="${fill}"/>`;
const polygon = (points: [number, number][], fill: string) =>
  `<path d="${points.map(([x, y], index) => `${index === 0 ? "M" : "L"}${n(x)} ${n(y)}`).join(" ")} Z" fill="${fill}"/>`;

/** A rectangle with round corners, from two rectangles and four circles. */
function roundRect(x: number, y: number, width: number, height: number, r: number, fill: string): string[] {
  const shapes = [rect(x + r, y, width - r * 2, height, fill)];
  // A pill has corners as tall as itself: no side rectangle, and two circles.
  if (height > r * 2) shapes.push(rect(x, y + r, width, height - r * 2, fill));
  const corners = [
    circle(x + r, y + r, r, fill),
    circle(x + width - r, y + r, r, fill),
    circle(x + r, y + height - r, r, fill),
    circle(x + width - r, y + height - r, r, fill),
  ];
  return [...shapes, ...new Set(corners)];
}

/** `text` as outlines, centered in the box. Returns the path and the text's width. */
function label(text: string, size: number, tracking: number, box: [x: number, y: number, w: number, h: number]) {
  const scale = size / font.unitsPerEm;
  const glyphs = font.stringToGlyphs(text);
  const width =
    glyphs.reduce((sum, glyph) => sum + (glyph.advanceWidth ?? 0) * scale, 0) + tracking * (glyphs.length - 1);
  if (width > box[2]) throw new Error(`icons: "${text}" is ${n(width)} wide, its box is ${box[2]}`);
  // Center on the capital height, so letters sit optically in the middle.
  const capHeight = (font.tables.os2?.sCapHeight ?? font.unitsPerEm * 0.7) * scale;
  const baseline = box[1] + (box[3] + capHeight) / 2;
  let x = box[0] + (box[2] - width) / 2;
  const parts: string[] = [];
  for (const glyph of glyphs) {
    parts.push(glyph.getPath(x, baseline, size).toPathData(2));
    x += (glyph.advanceWidth ?? 0) * scale + tracking;
  }
  return `<path d="${parts.join(" ")}" fill="${INK}"/>`;
}

function svg(width: number, height: number, shapes: string[]): string {
  return [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">`,
    ...shapes.map((shape) => `  ${shape}`),
    "</svg>",
    "",
  ].join("\n");
}

/** A round face button with its symbol. */
const face = (symbol: string[]) => svg(16, 16, [circle(8, 8, 7.5, FACE), ...symbol]);

/** A bar of the cross symbol: a thin rectangle through the center, rotated 45 degrees. */
function diagonal(direction: 1 | -1): string {
  const half = 3.5;
  const thick = 0.62;
  const along = (t: number, side: number): [number, number] => [
    8 + t * 0.7071 - side * 0.7071 * direction,
    8 + t * 0.7071 * direction + side * 0.7071,
  ];
  return polygon([along(-half, -thick), along(half, -thick), along(half, thick), along(-half, thick)], CROSS);
}

/** An equilateral triangle around (8, 8.7), `radius` from center to corner. */
function triangle(radius: number, fill: string): string {
  const corner = (angle: number): [number, number] => [8 + radius * Math.sin(angle), 8.7 - radius * Math.cos(angle)];
  return polygon([corner(0), corner((Math.PI * 2) / 3), corner((Math.PI * 4) / 3)], fill);
}

/** D-pad: a plus of four arms; `lit` names the arms drawn in the ink color. */
function dpad(lit: ("up" | "down" | "left" | "right")[]): string {
  const arm = (name: "up" | "down" | "left" | "right", x: number, y: number, w: number, h: number) =>
    rect(x, y, w, h, lit.includes(name) ? INK : FACE);
  return svg(16, 16, [
    ...roundRect(5.5, 1, 5, 14, 1.2, FACE),
    ...roundRect(1, 5.5, 14, 5, 1.2, FACE),
    arm("up", 6.6, 2, 2.8, 3.2),
    arm("down", 6.6, 10.8, 2.8, 3.2),
    arm("left", 2, 6.6, 3.2, 2.8),
    arm("right", 10.8, 6.6, 3.2, 2.8),
  ]);
}

/**
 * A shoulder button, 24 wide in a 32-wide texture. On the Vita these sit on the
 * top corners of the console, so each one curves away on its outer side: the L
 * button's top-left corner is a long sweep, and R is its mirror image. The
 * letter sits toward the square inner side.
 */
function shoulder(letter: "L" | "R"): string {
  // Outline of L, clockwise from the end of the sweep on the top edge.
  const outline: [command: string, ...points: number[]][] = [
    ["M", 11, 2],
    ["L", 21, 2],
    ["Q", 24, 2, 24, 5],
    ["L", 24, 11],
    ["Q", 24, 14, 21, 14],
    ["L", 3, 14],
    ["Q", 0, 14, 0, 11],
    ["C", 0, 6, 4.5, 2, 11, 2],
  ];
  // R mirrors every x coordinate across the middle of the 24-wide shape.
  const flip = letter === "R";
  const d = outline
    .map(([command, ...points]) => {
      const mapped = points.map((value, index) => (flip && index % 2 === 0 ? 24 - value : value));
      return `${command}${mapped.map(n).join(" ")}`;
    })
    .join(" ");
  const box: [number, number, number, number] = flip ? [0, 2, 21, 12] : [3, 2, 21, 12];
  return svg(32, 16, [`<path d="${d} Z" fill="${FACE}"/>`, label(letter, 9, 0, box)]);
}

/** SELECT and START: a pill `width` wide in a 64-wide texture. */
const pill = (text: string, width: number) =>
  svg(64, 16, [...roundRect(0, 2, width, 12, 6, FACE), label(text, 7, 0.5, [4, 2, width - 8, 12])]);

/** A bar `thick` wide from `a` to `b`, as a four-sided polygon. */
function segment(a: [number, number], b: [number, number], thick: number, fill: string): string {
  const length = Math.hypot(b[0] - a[0], b[1] - a[1]);
  const nx = (-(b[1] - a[1]) / length) * (thick / 2);
  const ny = ((b[0] - a[0]) / length) * (thick / 2);
  return polygon(
    [
      [a[0] + nx, a[1] + ny],
      [b[0] + nx, b[1] + ny],
      [b[0] - nx, b[1] - ny],
      [a[0] - nx, a[1] - ny],
    ],
    fill,
  );
}

// Status bar symbols are drawn in the top left 12x12 of a 16x16 texture; the
// UI clips the rest (components/icons.tsx). They come in two inks: light for the dark themes, dark for
// Daylight ("-day" files). Unlit parts of the Wi-Fi symbol use the faint ink.
const STATUS_INK = { night: { lit: "#dfe4ec", unlit: "#4a5266" }, day: { lit: "#334155", unlit: "#b4c0d0" } };
type Ink = keyof typeof STATUS_INK;

/**
 * The Vita's Wi-Fi symbol: a dot in the lower left corner and three quarter
 * rings around it, lit from the inside out. `level` is the rings lit (0 to 3).
 * The status bar shows it only while connected.
 */
function wifi(level: number, ink: Ink): string {
  const { lit, unlit } = STATUS_INK[ink];
  const [cx, cy] = [2, 10];
  // A quarter ring from straight up to straight right, as a polygon.
  const ring = (inner: number, outer: number, fill: string) => {
    const steps = 12;
    const at = (r: number, i: number): [number, number] => {
      const angle = -Math.PI / 2 + (i / steps) * (Math.PI / 2);
      return [cx + r * Math.cos(angle), cy + r * Math.sin(angle)];
    };
    const points: [number, number][] = [];
    for (let i = 0; i <= steps; i++) points.push(at(outer, i));
    for (let i = steps; i >= 0; i--) points.push(at(inner, i));
    return polygon(points, fill);
  };
  const rings = [
    [3.4, 4.8],
    [5.9, 7.3],
    [8.4, 9.8],
  ].map(([inner, outer], index) => ring(inner!, outer!, index < level ? lit : unlit));
  return svg(16, 16, [circle(cx, cy, 1.7, lit), ...rings]);
}

/** The Bluetooth rune: a spine and two arrowheads, with round joints. */
function bluetooth(ink: Ink): string {
  const SOFT = STATUS_INK[ink].lit;
  const thick = 1.2;
  const top: [number, number] = [6, 1];
  const bottom: [number, number] = [6, 11];
  const upper: [number, number] = [8.7, 3.6];
  const lower: [number, number] = [8.7, 8.4];
  const left = (y: number): [number, number] => [3.3, y];
  const joints = [top, bottom, upper, lower].map(([x, y]) => circle(x, y, thick / 2, SOFT));
  return svg(16, 16, [
    segment(top, bottom, thick, SOFT),
    segment(top, upper, thick, SOFT),
    segment(upper, left(8.4), thick, SOFT),
    segment(bottom, lower, thick, SOFT),
    segment(lower, left(3.6), thick, SOFT),
    ...joints,
  ]);
}

const FILES: Record<string, string> = {
  "btn-circle.svg": face([circle(8, 8, 4.1, CIRCLE), circle(8, 8, 2.9, FACE)]),
  "btn-cross.svg": face([diagonal(1), diagonal(-1)]),
  "btn-triangle.svg": face([triangle(4.7, TRIANGLE), triangle(2.5, FACE)]),
  "btn-square.svg": face([rect(4.5, 4.5, 7, 7, SQUARE), rect(5.7, 5.7, 4.6, 4.6, FACE)]),
  "btn-l.svg": shoulder("L"),
  "btn-r.svg": shoulder("R"),
  "btn-select.svg": pill("SELECT", 40),
  "btn-start.svg": pill("START", 36),
  "dpad-vertical.svg": dpad(["up", "down"]),
  "dpad-horizontal.svg": dpad(["left", "right"]),
  "arrow-left.svg": svg(8, 16, [polygon([[6.5, 4], [1.5, 8], [6.5, 12]], SOFT)]),
  "arrow-right.svg": svg(8, 16, [polygon([[1.5, 4], [6.5, 8], [1.5, 12]], SOFT)]),
  "plus.svg": svg(16, 16, [rect(7, 3.5, 2, 9, SOFT), rect(3.5, 7, 9, 2, SOFT)]),
  "bluetooth.svg": bluetooth("night"),
  "bluetooth-day.svg": bluetooth("day"),
  ...Object.fromEntries(
    (["night", "day"] as const).flatMap((ink) =>
      [0, 1, 2, 3].map((level) => [
        `wifi-${level}${ink === "day" ? "-day" : ""}.svg`,
        wifi(level, ink),
      ]),
    ),
  ),
};

mkdirSync(OUT, { recursive: true });
for (const [name, content] of Object.entries(FILES)) {
  await Bun.write(join(OUT, name), content);
  console.log(`icons/${name}  ${content.length} bytes`);
}
