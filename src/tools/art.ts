// Draws Pocket Shelf's LiveArea artwork into src/vita/sce_sys/, and writes
// the LiveArea layout:
//
//   icon0.png                         128x128  the bubble on the home screen
//   pic0.png                          960x544  the splash the system shows
//                                               while the app loads
//   livearea/contents/bg.png          840x500  the LiveArea background
//   livearea/contents/startup.png     280x158  the gate image
//   livearea/contents/template.xml             the layout: the app's name,
//                                               author and version as text
//
// The pictures hold no text. The system draws its "Start" label over the
// bottom of the gate image, which hid a name drawn there; the name, author
// and version are LiveArea text instead (the name and version from
// pocket.json, the author from src/about.json), in the
// `psmobile` style's frames as VitaShell's LiveArea uses them: the name in
// frame2, "by <author>" in frame3, "v<version>" in frame4.
//
//   bun run shelf:art
//
// The artwork repeats the app's own Home view in small (`MARK`): a selected
// tile with the white selection ring and smaller tiles after it. The Vita accepts these files as indexed
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
// Credits the manifest has no field for: src/about.json.
const ABOUT = (await Bun.file(join(SRC, "about.json")).json()) as { author?: string };

GlobalFonts.registerFromPath(join(SRC, "fonts/Figtree-Bold.ttf"), "Shelf Bold");

// The Midnight theme (src/themes.ts), the app's default.
const BG_TOP = "#101a36";
const BG_BOTTOM = "#02040c";

/**
 * The mark, in the app's logical pixels (the screen is 480x272, drawn at 2x):
 * the Home view in small. A selected tile with the white selection ring, and
 * two smaller tiles after it, aligned at the top. `components/scan-page.tsx`
 * draws the same mark with views; keep the two in step.
 */
const MARK = {
  width: 110,
  height: 48,
  ring: 2,
  /** The selected tile: its left, top and side; the ring is 2 px clear of it. */
  tile: { x: 4, y: 4, side: 40, colors: ["#a78bfa", "#5b21b6"] as [string, string] },
  small: [
    { x: 52, y: 4, side: 26, colors: ["#fbbf24", "#dc2626"] as [string, string] },
    { x: 84, y: 4, side: 26, colors: ["#34d399", "#0f766e"] as [string, string] },
  ],
  smallOpacity: 0.8,
};

function background(ctx: SKRSContext2D, width: number, height: number): void {
  const fill = ctx.createLinearGradient(0, 0, 0, height);
  fill.addColorStop(0, BG_TOP);
  fill.addColorStop(1, BG_BOTTOM);
  ctx.fillStyle = fill;
  ctx.fillRect(0, 0, width, height);
}

/** A soft round glow of `color`, strongest at the center. */
function glow(ctx: SKRSContext2D, x: number, y: number, radius: number, color: string, strength: number): void {
  const light = ctx.createRadialGradient(x, y, 0, x, y, radius);
  const alpha = (value: number) =>
    Math.round(value * 255)
      .toString(16)
      .padStart(2, "0");
  light.addColorStop(0, `${color}${alpha(strength)}`);
  light.addColorStop(1, `${color}00`);
  ctx.fillStyle = light;
  ctx.fillRect(x - radius, y - radius, radius * 2, radius * 2);
}

function roundedRect(ctx: SKRSContext2D, x: number, y: number, w: number, h: number, r: number): void {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

/** One tile: a vertical gradient, and a lighter wedge across its top corner. */
function tile(ctx: SKRSContext2D, x: number, y: number, side: number, colors: [string, string], opacity: number): void {
  ctx.globalAlpha = opacity;
  const fill = ctx.createLinearGradient(0, y, 0, y + side);
  fill.addColorStop(0, colors[0]);
  fill.addColorStop(1, colors[1]);
  ctx.fillStyle = fill;
  roundedRect(ctx, x, y, side, side, side * 0.08);
  ctx.fill();
  ctx.save();
  roundedRect(ctx, x, y, side, side, side * 0.08);
  ctx.clip();
  ctx.fillStyle = "#ffffff";
  ctx.globalAlpha = opacity * 0.16;
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(x + side, y);
  ctx.lineTo(x, y + side * 0.7);
  ctx.closePath();
  ctx.fill();
  ctx.restore();
  ctx.globalAlpha = 1;
}

/**
 * The mark with its top left at (`left`, `top`), each logical pixel `scale`
 * picture pixels. `extra` more small tiles continue the row to the right,
 * fading out, for pictures where it runs off the edge.
 */
function mark(ctx: SKRSContext2D, left: number, top: number, scale: number, extra = 0): void {
  const at = (value: number) => value * scale;
  const { tile: big, small, ring } = MARK;
  glow(ctx, left + at(big.x + big.side / 2), top + at(big.y + big.side / 2), at(big.side * 1.6), "#8b5cf6", 0.35);
  const smalls = [...small];
  for (let step = 0; step < extra; step++) {
    const last = smalls[smalls.length - 1]!;
    const next = MARK.small[(smalls.length) % MARK.small.length]!;
    smalls.push({ ...next, x: last.x + last.side + 6 });
  }
  for (const [index, item] of smalls.entries()) {
    const fade = index < small.length ? 1 : Math.max(0.25, 1 - (index - small.length + 1) * 0.3);
    tile(ctx, left + at(item.x), top + at(item.y), at(item.side), item.colors, MARK.smallOpacity * fade);
  }
  tile(ctx, left + at(big.x), top + at(big.y), at(big.side), big.colors, 1);
  // The ring: 2 px wide, 2 px clear of the tile, slightly rounded, as `SelectedFrame` draws it.
  const inset = at(ring / 2);
  ctx.strokeStyle = "#ffffff";
  ctx.lineWidth = at(ring);
  roundedRect(ctx, left + inset, top + inset, at(big.side + 8) - inset * 2, at(big.side + 8) - inset * 2, at(3));
  ctx.stroke();
}

function icon(): Uint8ClampedArray {
  const canvas = createCanvas(128, 128);
  const ctx = canvas.getContext("2d");
  background(ctx, 128, 128);
  // The home screen crops the icon to a circle. The selected tile sits just
  // left of the middle and the row runs off the right edge, as in the app.
  const scale = 1.3;
  mark(ctx, 64 - (MARK.tile.x + MARK.tile.side / 2) * scale - 10, 64 - (MARK.height / 2) * scale, scale, 1);
  return ctx.getImageData(0, 0, 128, 128).data;
}

function liveAreaBackground(): Uint8ClampedArray {
  const canvas = createCanvas(840, 500);
  const ctx = canvas.getContext("2d");
  background(ctx, 840, 500);
  // Large soft glows in the tiles' colors, and the row faint along the
  // bottom. The gate covers the middle and the system writes the name over
  // the left, so the picture stays dark there.
  glow(ctx, 700, 90, 320, "#8b5cf6", 0.28);
  glow(ctx, 120, 440, 300, "#06b6d4", 0.18);
  ctx.globalAlpha = 0.35;
  mark(ctx, 470, 360, 2.4, 3);
  ctx.globalAlpha = 1;
  ctx.fillStyle = "#02040c55";
  ctx.fillRect(0, 0, 840, 500);
  return ctx.getImageData(0, 0, 840, 500).data;
}

/**
 * The splash. It leads into the app's first screen: the Midnight theme's
 * background, and the mark and the name where the scan page of a first start
 * puts them (`SPLASH_MARK_TOP`, `SPLASH_TITLE_Y`, measured in the preview).
 */
const SPLASH_MARK_TOP = 56;
const SPLASH_TITLE_Y = 138;

function splash(): Uint8ClampedArray {
  const canvas = createCanvas(960, 544);
  const ctx = canvas.getContext("2d");
  background(ctx, 960, 544);
  mark(ctx, 480 - MARK.width, SPLASH_MARK_TOP * 2, 2);
  ctx.fillStyle = "#ffffff";
  ctx.font = '40px "Shelf Bold"';
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(MANIFEST.title, 480, SPLASH_TITLE_Y * 2);
  return ctx.getImageData(0, 0, 960, 544).data;
}

function startup(): Uint8ClampedArray {
  const canvas = createCanvas(280, 158);
  const ctx = canvas.getContext("2d");
  background(ctx, 280, 158);
  // No text: the system's "Start" label covers the bottom of the gate. The
  // mark sits above the middle, clear of that label.
  const scale = 1.2;
  mark(ctx, 140 - (MARK.width * scale) / 2, 26, scale);
  return ctx.getImageData(0, 0, 280, 158).data;
}

/** "0.1.0" as "0.1": the patch number only when it is not zero. */
function shortVersion(version: string): string {
  return version.replace(/^(\d+\.\d+)\.0$/, "$1");
}

function escapeXml(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** One line of LiveArea text in a frame, white with a shadow. */
function frame(id: string, text: string, size: number, attributes: string, bold = false): string {
  return `  <frame id="${id}">
    <liveitem>
      <text ${attributes}>
        <str color="#ffffff" size="${size}"${bold ? ' bold="on"' : ""} shadow="on">${escapeXml(text)}</str>
      </text>
    </liveitem>
  </frame>
`;
}

/**
 * The LiveArea layout: the name large in frame2, "by <author>" under it in
 * frame3 (left out without an author), the version smaller in frame4.
 */
function template(): string {
  const frames = [
    frame("frame2", MANIFEST.title, 50, 'valign="bottom" align="left" text-align="left" text-valign="bottom" line-space="3" ellipsis="on"', true),
    ABOUT.author
      ? frame("frame3", `by ${ABOUT.author}`, 22, 'valign="top" align="left" text-align="left" text-valign="top" line-space="2" ellipsis="on"')
      : "",
    frame("frame4", `v${shortVersion(MANIFEST.version)}`, 18, 'align="left" text-align="left" word-wrap="off" ellipsis="on"'),
  ];
  return `<?xml version="1.0" encoding="utf-8"?>
<livearea style="psmobile" format-ver="01.00" content-rev="1">
  <livearea-background>
    <image>bg.png</image>
  </livearea-background>
  <gate>
    <startup-image>startup.png</startup-image>
  </gate>
${frames.join("")}</livearea>
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

/** The most rounding error a pixel carries from its neighbors, per channel. */
const ERROR_CAP = 12;

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
  // Floyd-Steinberg dithering: each pixel's rounding error goes to the
  // pixels right of and below it. Without it, 256 colors showed the
  // backgrounds' gradients and glows as bands and blotches.
  const scanlines = Buffer.alloc((width + 1) * height);
  const error = new Float32Array(width * height * 3);
  for (let y = 0; y < height; y++) {
    scanlines[y * (width + 1)] = 0; // filter: none
    for (let x = 0; x < width; x++) {
      const at = y * width + x;
      const wanted = [0, 1, 2].map((channel) => {
        // The carried error is capped: uncapped, it built up in the dark
        // lower half of the splash into stray reddish dots.
        const carried = Math.max(-ERROR_CAP, Math.min(ERROR_CAP, error[at * 3 + channel]));
        const value = ((pixels[at] >> (16 - channel * 8)) & 255) + carried;
        return Math.max(0, Math.min(255, Math.round(value)));
      });
      const index = lookup((wanted[0] << 16) | (wanted[1] << 8) | wanted[2]);
      scanlines[y * (width + 1) + 1 + x] = index;
      const chosen = colors[index];
      for (let channel = 0; channel < 3; channel++) {
        const miss = wanted[channel] - ((chosen >> (16 - channel * 8)) & 255);
        const spread = (dx: number, dy: number, share: number) => {
          const nx = x + dx;
          const ny = y + dy;
          if (nx < 0 || nx >= width || ny >= height) return;
          error[(ny * width + nx) * 3 + channel] += miss * share;
        };
        spread(1, 0, 7 / 16);
        spread(-1, 1, 3 / 16);
        spread(0, 1, 5 / 16);
        spread(1, 1, 1 / 16);
      }
    }
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
  ["pic0.png", 960, 544, splash],
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
console.log(
  `livearea/contents/template.xml  ${MANIFEST.title}, by ${ABOUT.author ?? "(no author)"}, v${shortVersion(MANIFEST.version)}`,
);
