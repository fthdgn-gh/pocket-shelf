// Headless preview: renders Pocket Shelf in the wasm simulator and writes PNG
// frames, so a layout change can be checked without a Vita or an emulator.
//
//   bun run shelf:preview            # every shot
//   bun run shelf:preview grid menu  # only shots whose name contains a word
//
// The simulator has no installed titles, so this script supplies the host
// side: a native app table of sample titles, generated icons, an in-memory
// launch op, and an in-memory data folder. Frames land in .pocket-build/validation/shelf-preview/.

import { createCanvas } from "@napi-rs/canvas";
import { mkdirSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { ACCENT_HAS_ALPHA } from "../accent.ts";
import { BTN, IMG_FLAG_LINEAR, PSM } from "../../contracts/spec/spec.ts";
import { createSimFsHost } from "../../hosts/sim/fs.ts";
import { bootWorld, type SimWorld } from "../../hosts/sim/sim.ts";
import { encodePNG } from "../../tools/png.ts";

const ROOT = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const BUNDLE_DIR = "shelf-preview";
const OUT = join(ROOT, ".pocket-build/validation/shelf-preview");
const W = 480;
const H = 272;
const SCALE = 2;

// Sample library. Ids follow the Vita prefixes the app sorts by: PCS* retail
// games, NPXS* system apps, anything else homebrew.
const TITLES: [id: string, title: string][] = [
  ["PCSA00069", "Gravity Daze"],
  ["PCSB00245", "Neon Drift"],
  ["PCSE00317", "Castle Siege Tactics"],
  ["PCSF00042", "Star Harbor"],
  ["PCSG00551", "Pixel Quest Chronicles"],
  ["PCSA00126", "Sky Runner"],
  ["PCSB00890", "Deep Blue"],
  ["PCSE00444", "Iron Fist Arena"],
  ["PCSH00213", "Moon Garden"],
  ["PCSA00777", "Turbo Kart Rally"],
  ["PCSB00101", "Shadow Temple"],
  ["PCSE00912", "Orbit Breaker"],
  ["PCSG00003", "Paper Kingdom"],
  ["NPXS10001", "Media Player"],
  ["NPXS10003", "Web Browser"],
  ["NPXS10004", "Photo Viewer"],
  ["NPXS10009", "Music Box"],
  ["VITASHELL", "VitaShell"],
  ["RETROVITA", "RetroArch"],
  ["SAVEMGR00", "Save Manager"],
  ["PLUGLOAD1", "Plugin Loader"],
  ["TINYSYNTH", "Tiny Synth"],
  // Adrenaline and two of its game bubbles; the app lists the first only.
  ["PSPEMUCFW", "Adrenaline"],
  ["PSPEMU001", "Crisis Core"],
  ["PSPEMU002", "Castlevania SOTN"],
  // A large category, to check that a long list scrolls like a short one.
  ...Array.from({ length: 120 }, (_, index): [string, string] => [
    `HBREW${String(index).padStart(4, "0")}`,
    `Homebrew Sample ${index + 1}`,
  ]),
];

const ICON_COLORS: [string, string][] = [
  ["#f59e0b", "#b91c1c"],
  ["#22d3ee", "#1d4ed8"],
  ["#34d399", "#0f766e"],
  ["#a78bfa", "#5b21b6"],
  ["#fb7185", "#9d174d"],
  ["#fde047", "#c2410c"],
  ["#93c5fd", "#312e81"],
];

/** Every third sample icon is a logo on a transparent background. */
const seeThrough = (index: number) => index % 3 === 1;

/** What the Vita host reports as the accent of a sample icon (hosts/vita/src/accent.rs). */
function iconAccent(index: number): number {
  const color = Number.parseInt(ICON_COLORS[index % ICON_COLORS.length][0].slice(1), 16);
  return seeThrough(index) ? ACCENT_HAS_ALPHA | color : color;
}

/** A 128x128 stand-in for a title's icon0.png: gradient, shape and initials. */
function iconPixels(index: number, title: string): Uint8Array {
  const canvas = createCanvas(128, 128);
  const ctx = canvas.getContext("2d");
  const [from, to] = ICON_COLORS[index % ICON_COLORS.length];
  if (seeThrough(index)) {
    // A ring and the initials in the icon's color, nothing behind them.
    ctx.strokeStyle = from;
    ctx.lineWidth = 12;
    ctx.beginPath();
    ctx.arc(64, 64, 50, 0, Math.PI * 2);
    ctx.stroke();
    ctx.fillStyle = from;
    ctx.font = "bold 46px sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    const letters = title
      .split(" ")
      .map((word) => word[0])
      .join("")
      .slice(0, 2)
      .toUpperCase();
    ctx.fillText(letters, 64, 67);
    return new Uint8Array(ctx.getImageData(0, 0, 128, 128).data.buffer);
  }
  const gradient = ctx.createLinearGradient(0, 0, 128, 128);
  gradient.addColorStop(0, from);
  gradient.addColorStop(1, to);
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, 128, 128);
  ctx.globalAlpha = 0.22;
  ctx.fillStyle = "#ffffff";
  ctx.beginPath();
  ctx.arc(24 + ((index * 37) % 80), 30 + ((index * 53) % 60), 46, 0, Math.PI * 2);
  ctx.fill();
  ctx.globalAlpha = 0.18;
  ctx.fillStyle = "#000000";
  ctx.beginPath();
  ctx.moveTo(0, 128);
  ctx.lineTo(128, 60 + ((index * 17) % 40));
  ctx.lineTo(128, 128);
  ctx.fill();
  ctx.globalAlpha = 1;
  ctx.fillStyle = "#ffffff";
  ctx.font = "bold 54px sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  const initials = title
    .split(" ")
    .map((word) => word[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
  ctx.fillText(initials, 64, 68);
  return new Uint8Array(ctx.getImageData(0, 0, 128, 128).data.buffer);
}

/** A 512x256 stand-in for a title's full-screen picture: sky, sun and hills in its colors. */
function backdropPixels(index: number): Uint8Array {
  const canvas = createCanvas(512, 256);
  const ctx = canvas.getContext("2d");
  const [from, to] = ICON_COLORS[index % ICON_COLORS.length];
  const sky = ctx.createLinearGradient(0, 0, 0, 256);
  sky.addColorStop(0, to);
  sky.addColorStop(1, from);
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, 512, 256);
  ctx.fillStyle = "#ffffff";
  ctx.globalAlpha = 0.55;
  ctx.beginPath();
  ctx.arc(330 + ((index * 41) % 120), 70 + ((index * 23) % 40), 34, 0, Math.PI * 2);
  ctx.fill();
  for (const [layer, shade] of [0.25, 0.45, 0.7].entries()) {
    ctx.globalAlpha = shade;
    ctx.fillStyle = "#04060f";
    ctx.beginPath();
    ctx.moveTo(0, 256);
    for (let x = 0; x <= 512; x += 16) {
      const wave = Math.sin((x + index * 53) / (38 + layer * 21)) * (26 - layer * 6);
      ctx.lineTo(x, 150 + layer * 30 + wave);
    }
    ctx.lineTo(512, 256);
    ctx.fill();
  }
  ctx.globalAlpha = 1;
  return new Uint8Array(ctx.getImageData(0, 0, 512, 256).data.buffer);
}

/** Upload RGBA as a bilinear-filtered texture, the way the Vita host does. */
function uploadImage(ops: Record<string, unknown>, pixels: Uint8Array, width: number, height: number): number {
  const entry = ops.uploadImgEntry as ((blob: Uint8Array) => number) | undefined;
  if (!entry) {
    const upload = ops.uploadTexture as (buf: Uint8Array, w: number, h: number, psm: number) => number;
    return upload(pixels, width, height, PSM.PSM_8888);
  }
  // IMG entry: u16 width, u16 height, u8 pixel format, u8 flags, 2 bytes of padding, then pixels.
  const blob = new Uint8Array(8 + pixels.length);
  const view = new DataView(blob.buffer);
  view.setUint16(0, width, true);
  view.setUint16(2, height, true);
  blob[4] = PSM.PSM_8888;
  blob[5] = IMG_FLAG_LINEAR;
  blob.set(pixels, 8);
  return entry(blob);
}

/** The API key the preview "saved": the fake SteamGridDB accepts this one only. */
const PREVIEW_KEY = "0123456789abcdef0123456789abcdef";

/**
 * A stand-in for the Vita host's HTTPS extras and for SteamGridDB: a request
 * finishes after a few polls with a canned reply.
 */
function installNet(ops: Record<string, unknown>): void {
  interface Request {
    polls: number;
    status: number;
    text: string;
    size: number;
  }
  const requests = new Map<number, Request>();
  let next = 1;
  const reply = (data: unknown) => JSON.stringify({ success: true, data });
  ops.__netGet = (url: string, authorization: string): number => {
    let status = 200;
    let text = reply([]);
    if (authorization !== `Bearer ${PREVIEW_KEY}`) {
      status = 401;
      text = JSON.stringify({ success: false, errors: ["Authentication Required"] });
    } else if (url.includes("/search/autocomplete/")) {
      const term = decodeURIComponent(url.split("/").pop() ?? "");
      text = reply([
        { id: 4501, name: term, release_date: 1339459200 },
        { id: 4502, name: `${term} 2`, release_date: 1484870400 },
        { id: 4503, name: `${term} Remastered`, release_date: 1454371200 },
        { id: 4504, name: `${term}: The Animation`, release_date: 0 },
      ]);
    } else if (url.includes("/icons/game/") || url.includes("/heroes/game/")) {
      const kind = url.includes("/icons/") ? "icon" : "hero";
      const count = kind === "icon" ? 14 : 6;
      text = reply(
        Array.from({ length: count }, (_, index) => ({
          id: (kind === "icon" ? 7000 : 9000) + index,
          url: `https://cdn2.steamgriddb.com/${kind}/${index}.png`,
        })),
      );
    }
    requests.set(next, { polls: 0, status, text, size: text.length });
    return next++;
  };
  ops.__netSave = (_url: string, _path: string): number => {
    requests.set(next, { polls: 0, status: 200, text: "", size: 480 * 1024 });
    return next++;
  };
  ops.__netState = (id: number): string => {
    const request = requests.get(id);
    if (!request) return "error unknown request";
    request.polls++;
    if (request.polls < 6) return `busy ${Math.round((request.size * request.polls) / 6)} ${request.size}`;
    return `done ${request.status} ${request.size}`;
  };
  ops.__netText = (id: number): string => requests.get(id)?.text ?? "";
  ops.__netClose = (id: number): void => {
    requests.delete(id);
  };
}

function installHost(ops: Record<string, unknown>): void {
  installNet(ops);
  // Like the Vita host, the first requests for a picture answer -2 ("still
  // decoding") and a later one answers with the handle.
  const asked = new Map<string, number>();
  const stillDecoding = (key: string): boolean => {
    const count = (asked.get(key) ?? 0) + 1;
    asked.set(key, count);
    return count < 4;
  };
  // A downloaded candidate gets a generated picture, numbered by its id.
  const art = new Map<string, number>();
  // Accent per texture handle, as the Vita host keeps it.
  const accents = new Map<number, number>();
  ops.__artAccent = (handle: number): number => accents.get(handle) ?? -1;
  ops.__artDir = "ux0:/data/PocketShelf/art";
  ops.__appArt = (name: string): number => {
    let handle = art.get(name);
    if (handle === undefined) {
      if (stillDecoding(`art:${name}`)) return -2;
      const index = Number(name.replace(/\D/g, "")) || 0;
      handle = uploadImage(ops, iconPixels(index, `S ${index % 100}`), 128, 128);
      accents.set(handle, iconAccent(index));
      art.set(name, handle);
    }
    return handle;
  };
  const handles = new Map<string, number>();
  ops.appTable = () =>
    JSON.stringify({
      kind: "native",
      apps: TITLES.map(([id, title]) => ({ output: id, id, title, installed: true })),
      current: "PBCF7609D",
      resume: null,
    });
  ops.appLaunch = (id: string): number => (TITLES.some(([known]) => known === id) ? 1 : 0);
  ops.appIcon = (id: string): number => {
    const index = TITLES.findIndex(([known]) => known === id);
    // Every fifth title has no icon, to show the fallback tile.
    if (index < 0 || index % 5 === 4) return -1;
    let handle = handles.get(id);
    if (handle === undefined) {
      if (stillDecoding(`icon:${id}`)) return -2;
      handle = uploadImage(ops, iconPixels(index, TITLES[index][1]), 128, 128);
      accents.set(handle, iconAccent(index));
      handles.set(id, handle);
    }
    return handle;
  };
  // Like the Vita host, a picture is uploaded on request. Every fourth title
  // has none, to show the fallback.
  const backdrops = new Map<string, number>();

  ops.__appBackdrop = (id: string, file = ""): number => {
    const title = TITLES.findIndex(([known]) => known === id);
    // A file from the backdrops folder is numbered by the digits in its name.
    const index = file ? Number(file.replace(/\D/g, "")) || 0 : title;
    if (title < 0 || (!file && index % 4 === 3)) return -1;
    const key = file || id;
    let handle = backdrops.get(key);
    if (handle === undefined) {
      if (stillDecoding(`backdrop:${key}`)) return -2;
      handle = uploadImage(ops, backdropPixels(index), 512, 256);
      backdrops.set(key, handle);
    }
    return handle;
  };
}

/**
 * One tap, a hold for N frames, a wait of N frames (mask 0), or a restart of
 * the app that keeps its saved files.
 */
type Step = number | [mask: number, frames: number] | "restart";

interface Shot {
  name: string;
  steps: Step[];
  /** Save an API key before the app starts. */
  key?: boolean;
  /** Language saved in the settings before the app starts. */
  language?: string;
}



const tap = (button: number, times = 1): Step[] => Array<Step>(times).fill(button);

// The screens with the most text, once per translated language.
const LANGUAGE_SHOTS: Shot[] = ["tr", "de", "fr", "es"].flatMap((language) => {
  const shot = (name: string, steps: Step[]): Shot => ({ name: `60-${language}-${name}`, language, steps });
  return [
    shot("shelf", [BTN.RIGHT]),
    // The language row is one press up from the first row.
    shot("menu", [BTN.SELECT, BTN.UP]),
    shot("editor", [BTN.RIGHT, BTN.TRIANGLE]),
    shot("reset", [BTN.RIGHT, BTN.TRIANGLE, BTN.UP, BTN.CIRCLE]),
    shot("categories", [BTN.SELECT, ...tap(BTN.DOWN, 6), BTN.CIRCLE, ...tap(BTN.DOWN, 3), BTN.SQUARE]),
    shot("hidden", [BTN.SELECT, ...tap(BTN.DOWN, 6), BTN.CIRCLE, ...tap(BTN.DOWN, 3), BTN.START]),
    shot("keyboard", [BTN.SQUARE, BTN.DOWN, BTN.RIGHT, BTN.CIRCLE]),
    // R twice: the accents page. Its first key, typed plain and shifted.
    shot("accents", [BTN.SQUARE, ...tap(BTN.RTRIGGER, 2), BTN.CIRCLE, BTN.LTRIGGER, BTN.CIRCLE]),
    shot("search", [BTN.SQUARE, BTN.DOWN, BTN.RIGHT, BTN.CIRCLE, BTN.UP, ...tap(BTN.RIGHT, 3), BTN.CIRCLE, BTN.START]),
    shot("no-match", [BTN.SQUARE, ...tap(BTN.CIRCLE, 2), BTN.START]),
    shot("launching", [BTN.RIGHT, BTN.CIRCLE]),
    shot("picker", [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 4), BTN.CIRCLE, BTN.DOWN]),
    shot("online-key", [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 5), BTN.CIRCLE]),
  ];
});

const SHOTS: Shot[] = [
  { name: "01-shelf", steps: tap(BTN.RIGHT, 3) },
  { name: "02-shelf-first", steps: [] },
  { name: "03-shelf-apps", steps: [BTN.RTRIGGER, BTN.RIGHT] },
  { name: "04-menu", steps: [...tap(BTN.RIGHT, 2), BTN.SELECT, BTN.DOWN] },
  // SELECT, down to "View", right once: grid.
  { name: "05-grid", steps: [BTN.SELECT, ...tap(BTN.DOWN, 2), BTN.RIGHT, BTN.SELECT, ...tap(BTN.RIGHT, 4), BTN.DOWN] },
  { name: "06-list", steps: [BTN.SELECT, ...tap(BTN.DOWN, 2), ...tap(BTN.RIGHT, 2), BTN.SELECT, ...tap(BTN.DOWN, 2)] },
  { name: "07-editor", steps: [BTN.RIGHT, BTN.TRIANGLE] },
  { name: "08-keyboard", steps: [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 2), BTN.CIRCLE, BTN.RIGHT, BTN.DOWN] },
  { name: "09-art-picker", steps: [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 3), BTN.CIRCLE] },
  { name: "10-categories", steps: [BTN.SELECT, ...tap(BTN.DOWN, 6), BTN.CIRCLE, BTN.DOWN] },
  // Icon box off: SELECT, down to "Icon box", right once.
  { name: "29-icon-box-off", steps: [BTN.SELECT, ...tap(BTN.DOWN, 5), BTN.RIGHT, BTN.SELECT, BTN.RIGHT] },
  { name: "30-menu-scrolled", steps: [BTN.SELECT, ...tap(BTN.DOWN, 6)] },
  // Backdrop off: SELECT, down to "Backdrop", right once.
  { name: "20-no-backdrop", steps: [BTN.SELECT, ...tap(BTN.DOWN, 4), BTN.RIGHT, BTN.SELECT, ...tap(BTN.RIGHT, 2)] },
  { name: "11-launching", steps: [...tap(BTN.RIGHT, 2), BTN.CIRCLE] },
  // Second theme, to check that colors come from the theme and not from literals.
  { name: "17-detailed", steps: [BTN.SELECT, ...tap(BTN.DOWN, 3), BTN.RIGHT, BTN.SELECT, ...tap(BTN.RIGHT, 4)] },
  {
    name: "18-detailed-grid",
    steps: [BTN.SELECT, ...tap(BTN.DOWN, 2), BTN.RIGHT, BTN.DOWN, BTN.RIGHT, BTN.SELECT, ...tap(BTN.RIGHT, 6)],
  },
  { name: "19-basic", steps: [BTN.SELECT, ...tap(BTN.DOWN, 3), BTN.LEFT, BTN.SELECT, ...tap(BTN.RIGHT, 4)] },
  {
    name: "21-list-detailed",
    steps: [BTN.SELECT, ...tap(BTN.DOWN, 2), ...tap(BTN.RIGHT, 2), BTN.DOWN, BTN.RIGHT, BTN.SELECT, ...tap(BTN.DOWN, 4)],
  },
  // Title editor, down to "SteamGridDB": no API key saved yet.
  { name: "22-online-key", steps: [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 5), BTN.CIRCLE] },
  // With a key: the search runs and lists games.
  { name: "23-online-games", key: true, steps: [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 5), BTN.CIRCLE, [0, 30]] },
  // Choose the first game: its icon candidates, third one on screen.
  {
    name: "24-online-icon",
    key: true,
    steps: [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 5), BTN.CIRCLE, [0, 30], BTN.CIRCLE, [0, 40], BTN.RIGHT, [0, 20], BTN.RIGHT, [0, 20]],
  },
  // The backdrop row, applied: it shows behind the screen once the drawers close.
  {
    name: "25-online-backdrop",
    key: true,
    steps: [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 5), BTN.CIRCLE, [0, 30], BTN.CIRCLE, [0, 40], BTN.DOWN, [0, 20], BTN.RIGHT, [0, 20], BTN.CIRCLE],
  },
  {
    name: "26-online-applied",
    key: true,
    steps: [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 5), BTN.CIRCLE, [0, 30], BTN.CIRCLE, [0, 40], BTN.CIRCLE, BTN.DOWN, [0, 20], BTN.CIRCLE, ...tap(BTN.CROSS, 3), [0, 30]],
  },
  { name: "27-editor-rows", steps: [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 4)] },
  { name: "28-backdrop-picker", steps: [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 4), BTN.CIRCLE, BTN.DOWN] },
  // Dynamic theme: one step left of Midnight. The colors follow the selected title.
  { name: "31-dynamic", steps: [BTN.SELECT, BTN.LEFT, BTN.SELECT, [0, 30]] },
  { name: "32-dynamic-next", steps: [BTN.SELECT, BTN.LEFT, BTN.SELECT, ...tap(BTN.RIGHT, 2), [0, 40]] },
  { name: "33-dynamic-menu", steps: [BTN.SELECT, BTN.LEFT, BTN.SELECT, ...tap(BTN.RIGHT, 4), [0, 40], BTN.SELECT, BTN.DOWN] },
  {
    name: "34-dynamic-list",
    steps: [BTN.SELECT, BTN.LEFT, ...tap(BTN.DOWN, 2), ...tap(BTN.RIGHT, 2), ...tap(BTN.DOWN, 2), BTN.RIGHT, BTN.SELECT, ...tap(BTN.DOWN, 5), [0, 40]],
  },
  // Deep into a category of 125 titles, in each view.
  { name: "35-long-shelf", steps: [...tap(BTN.RTRIGGER, 2), [BTN.RIGHT, 420], [0, 30]] },
  {
    name: "36-long-grid",
    steps: [BTN.SELECT, ...tap(BTN.DOWN, 2), BTN.RIGHT, BTN.SELECT, ...tap(BTN.RTRIGGER, 2), [BTN.DOWN, 150], BTN.RIGHT, [0, 30]],
  },
  {
    name: "37-long-list",
    steps: [BTN.SELECT, ...tap(BTN.DOWN, 2), ...tap(BTN.RIGHT, 2), BTN.SELECT, ...tap(BTN.RTRIGGER, 2), [BTN.DOWN, 300], BTN.UP, [0, 30]],
  },
  // Holding RIGHT for a second repeats the move.
  { name: "12-hold-repeat", steps: [[BTN.RIGHT, 60]] },
  // Launch from another category, start again: the selection comes back.
  { name: "16-restored", steps: [...tap(BTN.RTRIGGER, 2), ...tap(BTN.RIGHT, 2), BTN.CIRCLE, "restart"] },
  { name: "13-theme", steps: [BTN.SELECT, BTN.RIGHT, BTN.SELECT, ...tap(BTN.RIGHT, 5)] },
  { name: "14-theme-light", steps: [BTN.SELECT, ...tap(BTN.LEFT, 2), BTN.SELECT, ...tap(BTN.RIGHT, 5)] },
  // Mark the second and third titles as favorites, then go to the new tab.
  {
    name: "38-favorites",
    steps: [BTN.RIGHT, BTN.TRIANGLE, BTN.CIRCLE, BTN.CROSS, BTN.RIGHT, BTN.TRIANGLE, BTN.CIRCLE, BTN.CROSS, BTN.LTRIGGER],
  },
  { name: "39-favorite-row", steps: [BTN.RIGHT, BTN.TRIANGLE, BTN.CIRCLE] },
  // Start three titles, start again: "Last Played" lists them, newest first.
  {
    name: "40-last-played",
    steps: [BTN.CIRCLE, ...tap(BTN.RIGHT, 2), BTN.CIRCLE, BTN.RTRIGGER, BTN.CIRCLE, "restart", ...tap(BTN.LTRIGGER, 2)],
  },
  // The same tab in the monospace font, the widest of the three.
  {
    name: "41-last-played-mono",
    steps: [BTN.CIRCLE, BTN.SELECT, BTN.DOWN, BTN.RIGHT, BTN.SELECT, BTN.LTRIGGER],
  },
  // Square, then "s" and "t" on the keyboard: the field counts the matches.
  { name: "42-search-typing", steps: [BTN.SQUARE, BTN.DOWN, BTN.RIGHT, BTN.CIRCLE, BTN.UP, ...tap(BTN.RIGHT, 3), BTN.CIRCLE] },
  {
    name: "43-search-results",
    steps: [BTN.SQUARE, BTN.DOWN, BTN.RIGHT, BTN.CIRCLE, BTN.UP, ...tap(BTN.RIGHT, 3), BTN.CIRCLE, BTN.START, BTN.RIGHT],
  },
  // Square on the search tab: the keyboard opens with the whole term.
  {
    name: "47-search-again",
    steps: [BTN.SQUARE, BTN.DOWN, BTN.RIGHT, BTN.CIRCLE, BTN.UP, ...tap(BTN.RIGHT, 3), BTN.CIRCLE, BTN.START, BTN.SQUARE],
  },
  // "qq" finds nothing.
  { name: "44-search-none", steps: [BTN.SQUARE, ...tap(BTN.CIRCLE, 2), BTN.START] },
  // Closing the search returns to the title that was selected before it.
  {
    name: "45-search-closed",
    steps: [BTN.RTRIGGER, ...tap(BTN.RIGHT, 2), BTN.SQUARE, BTN.DOWN, BTN.RIGHT, BTN.CIRCLE, BTN.START, BTN.CROSS],
  },
  // The footer's five hints in the monospace font, the widest of the three.
  {
    name: "46-search-mono",
    steps: [BTN.SELECT, BTN.DOWN, BTN.RIGHT, BTN.SELECT, BTN.SQUARE, BTN.DOWN, BTN.RIGHT, BTN.CIRCLE, BTN.START],
  },
  // A category named with letters from the accents page, kept across a restart.
  {
    name: "48-accent-category",
    steps: [
      BTN.SELECT, ...tap(BTN.DOWN, 6), BTN.CIRCLE, BTN.TRIANGLE,
      ...tap(BTN.RTRIGGER, 2), BTN.LTRIGGER, BTN.CIRCLE, BTN.DOWN, BTN.CIRCLE, BTN.DOWN, BTN.CIRCLE, BTN.START,
      "restart", BTN.SELECT, ...tap(BTN.DOWN, 6), BTN.CIRCLE, BTN.UP,
    ],
  },
  { name: "15-mono-font", steps: [BTN.SELECT, BTN.DOWN, BTN.RIGHT, BTN.SELECT, ...tap(BTN.RIGHT, 3)] },
  ...LANGUAGE_SHOTS,
];

function advance(world: SimWorld, mask: number, frames: number): void {
  for (let i = 0; i < frames; i++) {
    world.frame(mask);
    for (let t = 0; t < world.ticksPerFrame; t++) world.tick();
  }
}

/** Nodes in a DevTools tree snapshot. */
function countNodes(tree: unknown): number {
  if (!tree || typeof tree !== "object") return 0;
  const children = (tree as { k?: unknown[] }).k ?? [];
  return 1 + children.reduce<number>((sum, child) => sum + countNodes(child), 0);
}

async function capture(shot: Shot): Promise<void> {
  const files = createSimFsHost();
  if (shot.key) {
    (files.ns as { write(path: string, data: string, mode: number): number }).write(
      "steamgriddb.txt",
      JSON.stringify(PREVIEW_KEY),
      0,
    );
  }
  if (shot.language) {
    (files.ns as { write(path: string, data: string, mode: number): number }).write(
      "settings.json",
      // The payload is a JSON string holding the file's text.
      JSON.stringify(JSON.stringify({ language: shot.language })),
      0,
    );
  }
  const boot = async () => {
    const world = await bootWorld(`${BUNDLE_DIR}/main`, 60, { fs: files.ns }, installHost, {
      width: W,
      height: H,
      rasterDensity: SCALE,
      renderScale: SCALE,
    });
    advance(world, 0, 20);
    return world;
  };
  let world = await boot();
  for (const step of shot.steps) {
    if (step === "restart") {
      world = await boot();
      continue;
    }
    const [mask, frames] = typeof step === "number" ? [step, 2] : step;
    advance(world, mask, frames);
    advance(world, 0, 6);
  }
  // Let transitions and springs settle before the frame is read.
  advance(world, 0, 50);

  const file = join(OUT, `${shot.name}.png`);
  await Bun.write(file, encodePNG(world.render(), W * SCALE, H * SCALE));
  // The tree probe advances the world a frame, so it runs after the frame is read.
  files.dispose();
  console.log(`  ${file.slice(ROOT.length + 1)}  (${countNodes(world.getTree())} nodes)`);
}

const filters = Bun.argv.slice(2);
const shots = SHOTS.filter((shot) => filters.length === 0 || filters.some((word) => shot.name.includes(word)));
if (shots.length === 0) {
  console.error(`shelf preview: no shot matches ${filters.join(", ")}`);
  process.exit(1);
}

rmSync(join(ROOT, "dist", BUNDLE_DIR), { recursive: true, force: true });
const build = Bun.spawnSync(
  [process.execPath, "tools/build.ts", "src/main.tsx", `--outdir=dist/${BUNDLE_DIR}/`, `--density=${SCALE}`],
  { cwd: ROOT, stdout: "pipe", stderr: "inherit" },
);
if (build.exitCode !== 0) {
  console.error(build.stdout.toString());
  process.exit(build.exitCode ?? 1);
}

mkdirSync(OUT, { recursive: true });
console.log(`shelf preview: ${shots.length} shot(s)`);
for (const shot of shots) await capture(shot);
