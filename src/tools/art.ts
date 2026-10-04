// Draws Pocket Shelf's LiveArea artwork into src/vita/sce_sys/, and writes
// the LiveArea layout:
//
//   icon0.png                         128x128  the bubble on the home screen
//   livearea/contents/bg.png          840x500  the LiveArea background
//   livearea/contents/startup.png     280x158  the gate image
//   livearea/contents/template.xml             the layout: the app's name and
//                                               version as LiveArea text
//
// The pictures hold no text. The system draws its "Start" label over the
// bottom of the gate image, which hid a name drawn there; the name and the
// version are LiveArea text instead (the `psmobile` style's frame2 and
// frame3, as Adrenaline's LiveArea uses them), read from pocket.json.
//
//   bun run shelf:art
//
// The artwork repeats the app's own shelf: square tiles standing on a lit
// edge, the middle one selected. The Vita accepts these files as indexed
// 8-bit PNG only (tools/vita-package.ts checks it), so each drawing is
// reduced to at most 256 colors before it is written. The PNG files are
// committed; a build does not run this script.

import { createCanvas, GlobalFonts, type SKRSContext2D } from "@napi-rs/canvas";
import { mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync } from "node:zlib";

const SRC = resolve(fileURLToPath(new URL("..", import.meta.url)));
const OUT = join(SRC, "vita/sce_sys");
const MANIFEST = (await Bun.file(join(SRC, "../pocket.json")).json()) as { title: string; version: string };

GlobalFonts.registerFromPath(join(SRC, "fonts/SpaceGrotesk-Bold.ttf"), "Shelf Bold");

const BG_TOP = "#14204a";
const BG_BOTTOM = "#02040c";
const ACCENT = "#22d3ee";

/** Tile fills, top and bottom of a vertical gradient. */
const TILES: [string, string][] = [
  ["#fbbf24", "#dc2626"],
  ["#34d399", "#0f766e"],
  ["#a78bfa", "#5b21b6"],
  ["#38bdf8", "#4338ca"],
  ["#f472b6", "#be123c"],
  ["#67e8f9", "#1d4ed8"],
];

function background(ctx: SKRSContext2D, width: number, height: number): void {
  const fill = ctx.createLinearGradient(0, 0, 0, height);
  fill.addColorStop(0, BG_TOP);
  fill.addColorStop(1, BG_BOTTOM);
  ctx.fillStyle = fill;
  ctx.fillRect(0, 0, width, height);
}

/** One square tile standing on `floor`, centered on `centerX`. */
function tile(
  ctx: SKRSContext2D,
  centerX: number,
  floor: number,
  size: number,
  colors: [string, string],
  opacity: number,
  ring = 0,
): void {
  const x = Math.round(centerX - size / 2);
  const y = Math.round(floor - size);
  ctx.globalAlpha = opacity;
  const fill = ctx.createLinearGradient(0, y, 0, y + size);
  fill.addColorStop(0, colors[0]);
  fill.addColorStop(1, colors[1]);
  ctx.fillStyle = fill;
  ctx.fillRect(x, y, size, size);
  // A lighter wedge across the top corner, the way light falls on a cover.
  ctx.fillStyle = "#ffffff";
  ctx.globalAlpha = opacity * 0.16;
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(x + size, y);
  ctx.lineTo(x, y + size * 0.7);
  ctx.closePath();
  ctx.fill();
  ctx.globalAlpha = 1;
  if (ring > 0) {
    const gap = Math.max(1, Math.round(ring / 2));
    ctx.strokeStyle = ACCENT;
    ctx.lineWidth = ring;
    const offset = gap + ring / 2;
    ctx.strokeRect(x - offset, y - offset, size + offset * 2, size + offset * 2);
  }
}

/** The lit edge the tiles stand on, and its glow on the surface below. */
function shelf(ctx: SKRSContext2D, width: number, y: number, thickness: number, glow: number): void {
  const light = ctx.createLinearGradient(0, y, 0, y + glow);
  light.addColorStop(0, "#22d3ee55");
  light.addColorStop(1, "#22d3ee00");
  ctx.fillStyle = light;
  ctx.fillRect(0, y, width, glow);
  ctx.fillStyle = ACCENT;
  ctx.fillRect(0, y, width, thickness);
}

/**
 * A row of tiles around a selected one. `pitch` is the distance between tile
 * centers, `count` the number of tiles on each side of the middle.
 */
function row(
  ctx: SKRSContext2D,
  centerX: number,
  floor: number,
  size: number,
  pitch: number,
  count: number,
  ring: number,
  dim: number,
): void {
  for (let step = count; step >= 1; step--) {
    for (const side of [-1, 1]) {
      const index = (2 + side * step + TILES.length * 4) % TILES.length;
      tile(ctx, centerX + side * step * pitch, floor, size, TILES[index], dim);
    }
  }
  tile(ctx, centerX, floor, Math.round(size * 1.36), TILES[2], 1, ring);
}

function icon(): Uint8ClampedArray {
  const canvas = createCanvas(128, 128);
  const ctx = canvas.getContext("2d");
  background(ctx, 128, 128);
  // The home screen crops the icon to a circle, so the row runs off both
  // sides and the selected tile sits in the middle.
  const floor = 88;
  shelf(ctx, 128, floor + 5, 3, 34);
  row(ctx, 64, floor, 40, 54, 2, 3, 0.7);
  return ctx.getImageData(0, 0, 128, 128).data;
}

function liveAreaBackground(): Uint8ClampedArray {
  const canvas = createCanvas(840, 500);
  const ctx = canvas.getContext("2d");
  background(ctx, 840, 500);
  // The gate image covers the middle of the LiveArea, so the row sits low
  // and stays dim behind it.
  const floor = 404;
  shelf(ctx, 840, floor + 12, 4, 84);
  row(ctx, 420, floor, 96, 132, 4, 5, 0.42);
  // Darken the whole picture a little so the system's text stays readable.
  ctx.fillStyle = "#02040c66";
  ctx.fillRect(0, 0, 840, 500);
  return ctx.getImageData(0, 0, 840, 500).data;
}

function startup(): Uint8ClampedArray {
  const canvas = createCanvas(280, 158);
  const ctx = canvas.getContext("2d");
  background(ctx, 280, 158);
  // No text: the system's "Start" label covers the bottom of the gate. The
  // row sits a little above the middle, clear of that label.
  const floor = 100;
  shelf(ctx, 280, floor + 7, 2, 44);
  row(ctx, 140, floor, 46, 64, 2, 3, 0.6);
  return ctx.getImageData(0, 0, 280, 158).data;
}

/** "0.1.0" as "0.1": the patch number only when it is not zero. */
function shortVersion(version: string): string {
  return version.replace(/^(\d+\.\d+)\.0$/, "$1");
}

function escapeXml(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** The LiveArea layout: the name large in frame2, the version under it in frame3. */
function template(): string {
  return `<?xml version="1.0" encoding="utf-8"?>
<livearea style="psmobile" format-ver="01.00" content-rev="1">
  <livearea-background>
    <image>bg.png</image>
  </livearea-background>
  <gate>
    <startup-image>startup.png</startup-image>
  </gate>
  <frame id="frame2">
    <liveitem>
      <text valign="bottom" align="left" text-align="left" text-valign="bottom" line-space="3" ellipsis="on">
        <str color="#ffffff" size="50" bold="on" shadow="on">${escapeXml(MANIFEST.title)}</str>
      </text>
    </liveitem>
  </frame>
  <frame id="frame3">
    <liveitem>
      <text valign="top" align="left" text-align="left" text-valign="top" line-space="2" ellipsis="on">
        <str color="#ffffff" size="22" shadow="on">Version ${escapeXml(shortVersion(MANIFEST.version))}</str>
      </text>
    </liveitem>
  </frame>
</livearea>
`;
}

// --- Indexed PNG ------------------------------------------------------------

interface Box {
  colors: number[];
  min: [number, number, number];
  max: [number, number, number];
}

function boxOf(colors: number[]): Box {
  const min: [number, number, number] = [255, 255, 255];
  const max: [number, number, number] = [0, 0, 0];
  for (const color of colors) {
    for (let channel = 0; channel < 3; channel++) {
      const value = (color >> (16 - channel * 8)) & 255;
      if (value < min[channel]) min[channel] = value;
      if (value > max[channel]) max[channel] = value;
    }
  }
  return { colors, min, max };
}

/**
 * Median cut: split the box with the widest channel at its median pixel until
 * there are `limit` boxes, then average each one. `counts` weighs a color by
 * how many pixels use it.
 */
function palette(counts: Map<number, number>, limit: number): number[] {
  const boxes = [boxOf([...counts.keys()])];
  while (boxes.length < limit) {
    let widest = -1;
    let widestRange = 0;
    let widestChannel = 0;
    for (const [index, box] of boxes.entries()) {
      if (box.colors.length < 2) continue;
      for (let channel = 0; channel < 3; channel++) {
        const range = box.max[channel] - box.min[channel];
        if (range > widestRange) {
          widestRange = range;
          widest = index;
          widestChannel = channel;
        }
      }
    }
    if (widest < 0) break;
    const box = boxes[widest];
    const shift = 16 - widestChannel * 8;
    box.colors.sort((a, b) => ((a >> shift) & 255) - ((b >> shift) & 255));
    let total = 0;
    for (const color of box.colors) total += counts.get(color) ?? 0;
    let seen = 0;
    let cut = 1;
    for (let index = 0; index < box.colors.length - 1; index++) {
      seen += counts.get(box.colors[index]) ?? 0;
      cut = index + 1;
      if (seen * 2 >= total) break;
    }
    boxes.splice(widest, 1, boxOf(box.colors.slice(0, cut)), boxOf(box.colors.slice(cut)));
  }
  return boxes.map((box) => {
    let weight = 0;
    const sum = [0, 0, 0];
    for (const color of box.colors) {
      const count = counts.get(color) ?? 0;
      weight += count;
      sum[0] += ((color >> 16) & 255) * count;
      sum[1] += ((color >> 8) & 255) * count;
      sum[2] += (color & 255) * count;
    }
    const channel = (value: number) => Math.round(value / Math.max(1, weight));
    return (channel(sum[0]) << 16) | (channel(sum[1]) << 8) | channel(sum[2]);
  });
}

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

function chunk(type: string, data: Uint8Array): Buffer {
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  let crc = 0xffffffff;
  for (const byte of body) crc = CRC_TABLE[(crc ^ byte) & 255] ^ (crc >>> 8);
  const head = Buffer.alloc(4);
  head.writeUInt32BE(data.length, 0);
  const tail = Buffer.alloc(4);
  tail.writeUInt32BE((crc ^ 0xffffffff) >>> 0, 0);
  return Buffer.concat([head, body, tail]);
}

/** Encode opaque RGBA pixels as an indexed, non-interlaced 8-bit PNG. */
function indexedPng(rgba: Uint8ClampedArray, width: number, height: number): Buffer {
  const pixels = new Uint32Array(width * height);
  const counts = new Map<number, number>();
  for (let index = 0; index < pixels.length; index++) {
    const color = (rgba[index * 4] << 16) | (rgba[index * 4 + 1] << 8) | rgba[index * 4 + 2];
    pixels[index] = color;
    counts.set(color, (counts.get(color) ?? 0) + 1);
  }
  const colors = palette(counts, 256);
  const nearest = new Map<number, number>();
  const lookup = (color: number): number => {
    let found = nearest.get(color);
    if (found !== undefined) return found;
    let best = Infinity;
    found = 0;
    for (const [index, entry] of colors.entries()) {
      const dr = ((color >> 16) & 255) - ((entry >> 16) & 255);
      const dg = ((color >> 8) & 255) - ((entry >> 8) & 255);
      const db = (color & 255) - (entry & 255);
      const distance = dr * dr + dg * dg + db * db;
      if (distance < best) {
        best = distance;
        found = index;
      }
    }
    nearest.set(color, found);
    return found;
  };
  const scanlines = Buffer.alloc((width + 1) * height);
  for (let y = 0; y < height; y++) {
    scanlines[y * (width + 1)] = 0; // filter: none
    for (let x = 0; x < width; x++) scanlines[y * (width + 1) + 1 + x] = lookup(pixels[y * width + x]);
  }
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8; // bit depth
  header[9] = 3; // color type: indexed
  const table = Buffer.alloc(colors.length * 3);
  for (const [index, color] of colors.entries()) {
    table[index * 3] = (color >> 16) & 255;
    table[index * 3 + 1] = (color >> 8) & 255;
    table[index * 3 + 2] = color & 255;
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("PLTE", table),
    chunk("IDAT", deflateSync(scanlines, { level: 9 })),
    chunk("IEND", new Uint8Array(0)),
  ]);
}

const FILES: [path: string, width: number, height: number, draw: () => Uint8ClampedArray][] = [
  ["icon0.png", 128, 128, icon],
  ["livearea/contents/bg.png", 840, 500, liveAreaBackground],
  ["livearea/contents/startup.png", 280, 158, startup],
];

for (const [path, width, height, draw] of FILES) {
  const file = join(OUT, path);
  mkdirSync(dirname(file), { recursive: true });
  const png = indexedPng(draw(), width, height);
  await Bun.write(file, png);
  console.log(`${path}  ${width}x${height}  ${png.length} bytes`);
}
await Bun.write(join(OUT, "livearea/contents/template.xml"), template());
console.log(`livearea/contents/template.xml  ${MANIFEST.title}, version ${shortVersion(MANIFEST.version)}`);
