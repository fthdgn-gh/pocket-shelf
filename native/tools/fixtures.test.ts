// Writes the saved-data fixtures for the port's round-trip tests
// (native/core/tests/fixtures/) with the current app's own save functions,
// on an in-memory data folder. Run with `bun run next:fixtures`.

import { expect, mock, test } from "bun:test";
import { mkdirSync, writeFileSync as writeReal } from "node:fs";
import { dirname, join } from "node:path";

const files = new Map<string, string>();

mock.module("@pocketjs/framework/fs", () => ({
  readFileSync: (path: string) => {
    const text = files.get(path);
    if (text === undefined) throw new Error(`ENOENT ${path}`);
    return text;
  },
  writeFileSync: (path: string, text: string) => void files.set(path, text),
  readdirSync: (dir: string) =>
    [...files.keys()].filter((path) => path.startsWith(`${dir}/`)).map((path) => path.slice(dir.length + 1)),
  mkdirSync: () => {},
  rmSync: (path: string) => void files.delete(path),
}));

const OUT = join(import.meta.dir, "../core/tests/fixtures");

function dump(name: string, path: string) {
  const text = files.get(path);
  if (text === undefined) throw new Error(`not written: ${path}`);
  const out = join(OUT, name);
  mkdirSync(dirname(out), { recursive: true });
  writeReal(out, text);
}

test("writes the fixtures", async () => {
  const { saveSettings } = await import("../../src/settings.ts");
  const { saveCategoryConfig } = await import("../../src/categories.ts");
  const { saveRecent } = await import("../../src/recent.ts");
  const { saveOverrides } = await import("../../src/overrides.ts");

  saveSettings({
    language: "tr",
    theme: "dynamic",
    view: "cascade",
    detail: "detailed",
    confirm: "cross",
    backdrop: false,
    iconBox: true,
    statusBar: true,
    clock: "12",
    batteryPercent: false,
    updates: "beta",
    scraper: "libretro",
    category: "retro-snes",
    title: "ra0k3x9zq",
  });
  dump("settings/full.json", "settings.json");
  // No title selected: the key is left out.
  saveSettings({
    language: "en",
    theme: "midnight",
    view: "home",
    detail: "normal",
    confirm: "circle",
    backdrop: true,
    iconBox: true,
    statusBar: true,
    clock: "system",
    batteryPercent: true,
    updates: "off",
    scraper: "auto",
    category: "games",
    title: undefined,
  });
  dump("settings/no-title.json", "settings.json");

  saveCategoryConfig({
    custom: [
      { id: "emulators", label: "Emulators", custom: true },
      { id: "cocuk-oyunlari", label: "Çocuk \"Oyunları\"", custom: true },
    ],
    order: ["smart-recent", "games", "emulators", "retro-snes", "system"],
    hidden: ["builtin-psm"],
  });
  dump("categories/full.json", "categories.json");
  saveCategoryConfig({ custom: [], order: [], hidden: [] });
  dump("categories/empty.json", "categories.json");

  saveRecent(["PCSA00069", "ra0k3x9zq", "VITASHELL"]);
  dump("recent/three.json", "recent.json");
  saveRecent([]);
  dump("recent/empty.json", "recent.json");

  saveOverrides({
    PCSA00069: { category: "emulators", title: "Gravity \\ Daze", art: "PCSA00069-48213.png", favorite: true },
    PCSB00245: { art: "icon", backdrop: "none" },
    ra0k3x9zq: { title: "Pokémon – Kırmızı", backdrop: "ra0k3x9zq-lr-snap.png" },
  });
  for (const id of ["PCSA00069", "PCSB00245", "ra0k3x9zq"]) dump(`titles/${id}.json`, `titles/${id}.json`);
  dump("titles.cache.json", "titles.cache.json");
  expect(files.size).toBeGreaterThan(0);
});
