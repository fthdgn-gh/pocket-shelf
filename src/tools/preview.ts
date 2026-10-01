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

/** A 128x128 stand-in for a title's icon0.png: gradient, shape and initials. */
function iconPixels(index: number, title: string): Uint8Array {
  const canvas = createCanvas(128, 128);
  const ctx = canvas.getContext("2d");
  const [from, to] = ICON_COLORS[index % ICON_COLORS.length];
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

/** Upload 128x128 RGBA as a bilinear-filtered texture, the way the Vita host does. */
function uploadIcon(ops: Record<string, unknown>, pixels: Uint8Array): number {
  const entry = ops.uploadImgEntry as ((blob: Uint8Array) => number) | undefined;
  if (!entry) {
    const upload = ops.uploadTexture as (buf: Uint8Array, w: number, h: number, psm: number) => number;
    return upload(pixels, 128, 128, PSM.PSM_8888);
  }
  // IMG entry: u16 width, u16 height, u8 pixel format, u8 flags, 2 bytes of padding, then pixels.
  const blob = new Uint8Array(8 + pixels.length);
  const view = new DataView(blob.buffer);
  view.setUint16(0, 128, true);
  view.setUint16(2, 128, true);
  blob[4] = PSM.PSM_8888;
  blob[5] = IMG_FLAG_LINEAR;
  blob.set(pixels, 8);
  return entry(blob);
}

function installHost(ops: Record<string, unknown>): void {
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
      handle = uploadIcon(ops, iconPixels(index, TITLES[index][1]));
      handles.set(id, handle);
    }
    return handle;
  };
}

/** One tap, a hold for N frames, or a restart of the app that keeps its saved files. */
type Step = number | [mask: number, frames: number] | "restart";

interface Shot {
  name: string;
  steps: Step[];
}

const tap = (button: number, times = 1): Step[] => Array<Step>(times).fill(button);

const SHOTS: Shot[] = [
  { name: "01-shelf", steps: tap(BTN.RIGHT, 3) },
  { name: "02-shelf-first", steps: [] },
  { name: "03-shelf-apps", steps: [BTN.RTRIGGER, BTN.RIGHT] },
  { name: "04-menu", steps: [...tap(BTN.RIGHT, 2), BTN.SELECT, BTN.DOWN] },
  // SELECT, down to "View", right once: grid.
  { name: "05-grid", steps: [BTN.SELECT, ...tap(BTN.DOWN, 2), BTN.RIGHT, BTN.SELECT, ...tap(BTN.RIGHT, 4), BTN.DOWN] },
  { name: "06-list", steps: [BTN.SELECT, ...tap(BTN.DOWN, 2), ...tap(BTN.RIGHT, 2), BTN.SELECT, ...tap(BTN.DOWN, 2)] },
  { name: "07-editor", steps: [BTN.RIGHT, BTN.TRIANGLE] },
  { name: "08-keyboard", steps: [BTN.RIGHT, BTN.TRIANGLE, BTN.DOWN, BTN.CIRCLE, BTN.RIGHT, BTN.DOWN] },
  { name: "09-art-picker", steps: [BTN.RIGHT, BTN.TRIANGLE, ...tap(BTN.DOWN, 2), BTN.CIRCLE] },
  { name: "10-categories", steps: [BTN.SELECT, ...tap(BTN.DOWN, 4), BTN.CIRCLE, BTN.DOWN] },
  { name: "11-launching", steps: [...tap(BTN.RIGHT, 2), BTN.CIRCLE] },
  // Second theme, to check that colors come from the theme and not from literals.
  { name: "17-detailed", steps: [BTN.SELECT, ...tap(BTN.DOWN, 3), BTN.RIGHT, BTN.SELECT, ...tap(BTN.RIGHT, 4)] },
  {
    name: "18-detailed-grid",
    steps: [BTN.SELECT, ...tap(BTN.DOWN, 2), BTN.RIGHT, BTN.DOWN, BTN.RIGHT, BTN.SELECT, ...tap(BTN.RIGHT, 6)],
  },
  { name: "19-basic", steps: [BTN.SELECT, ...tap(BTN.DOWN, 3), BTN.LEFT, BTN.SELECT, ...tap(BTN.RIGHT, 4)] },
  // Holding RIGHT for a second repeats the move.
  { name: "12-hold-repeat", steps: [[BTN.RIGHT, 60]] },
  // Launch from another category, start again: the selection comes back.
  { name: "16-restored", steps: [...tap(BTN.RTRIGGER, 2), ...tap(BTN.RIGHT, 2), BTN.CIRCLE, "restart"] },
  { name: "13-theme", steps: [BTN.SELECT, BTN.RIGHT, BTN.SELECT, ...tap(BTN.RIGHT, 5)] },
  { name: "14-theme-light", steps: [BTN.SELECT, ...tap(BTN.LEFT, 1), BTN.SELECT, ...tap(BTN.RIGHT, 5)] },
  { name: "15-mono-font", steps: [BTN.SELECT, BTN.DOWN, BTN.RIGHT, BTN.SELECT, ...tap(BTN.RIGHT, 3)] },
];

function advance(world: SimWorld, mask: number, frames: number): void {
  for (let i = 0; i < frames; i++) {
    world.frame(mask);
    for (let t = 0; t < world.ticksPerFrame; t++) world.tick();
  }
}

async function capture(shot: Shot): Promise<void> {
  const files = createSimFsHost();
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
  files.dispose();
  console.log(`  ${file.slice(ROOT.length + 1)}`);
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
