import { PSP_ID, PSX_ID, CATEGORY_LABEL_MAX, type Category } from "./categories.ts";

/**
 * Categories of RetroArch games, one per console. RetroArch names a console by
 * its database (`Nintendo - Super Nintendo Entertainment System`), in each
 * playlist entry and in each playlist's file name; the host reports that name
 * as the game's `console` (hosts/vita/src/retroarch.rs). Games for the same
 * console share a category whichever core starts them.
 *
 * The ids are saved in `categories.json` (tab order, hidden tabs) and in the
 * titles' changes (a title moved into one), so an id never changes once
 * given out. They start with `retro-`. A category the user made earlier with
 * the same id (from a label such as "Retro NES") keeps it, and the console's
 * games go into it.
 */

interface Known {
  id: string;
  label: string;
}

const known = (id: string, label: string): Known => ({ id: `retro-${id}`, label });

/** Database names with a short label. Several databases may share a category. */
const KNOWN: Record<string, Known> = {
  "Nintendo - Nintendo Entertainment System": known("nes", "NES"),
  "Nintendo - Family Computer Disk System": known("fds", "Famicom Disk"),
  "Nintendo - Super Nintendo Entertainment System": known("snes", "SNES"),
  "Nintendo - Satellaview": known("satellaview", "Satellaview"),
  "Nintendo - Sufami Turbo": known("sufami", "Sufami Turbo"),
  "Nintendo - Nintendo 64": known("n64", "N64"),
  "Nintendo - Nintendo 64DD": known("n64dd", "N64DD"),
  "Nintendo - Game Boy": known("gb", "Game Boy"),
  "Nintendo - Game Boy Color": known("gbc", "Game Boy Color"),
  "Nintendo - Game Boy Advance": known("gba", "GBA"),
  "Nintendo - Nintendo DS": known("nds", "Nintendo DS"),
  "Nintendo - Virtual Boy": known("vb", "Virtual Boy"),
  "Nintendo - Pokemon Mini": known("pokemini", "Pokemon Mini"),
  "Sega - SG-1000": known("sg1000", "SG-1000"),
  "Sega - Master System - Mark III": known("sms", "Master System"),
  "Sega - Game Gear": known("gg", "Game Gear"),
  "Sega - Mega Drive - Genesis": known("md", "Mega Drive"),
  "Sega - Mega-CD - Sega CD": known("mcd", "Mega-CD"),
  "Sega - 32X": known("32x", "32X"),
  "Sega - PICO": known("pico", "Sega Pico"),
  "Sega - Saturn": known("saturn", "Saturn"),
  "Sega - Dreamcast": known("dc", "Dreamcast"),
  "NEC - PC Engine - TurboGrafx 16": known("pce", "PC Engine"),
  "NEC - PC Engine CD - TurboGrafx-CD": known("pcecd", "PC Engine CD"),
  "NEC - PC Engine SuperGrafx": known("sgx", "SuperGrafx"),
  "NEC - PC-FX": known("pcfx", "PC-FX"),
  "SNK - Neo Geo": known("neogeo", "Neo Geo"),
  // A playlist the user named, as RetroArch's playlist manager lets them.
  "Neo Geo": known("neogeo", "Neo Geo"),
  "SNK - Neo Geo CD": known("neogeocd", "Neo Geo CD"),
  "SNK - Neo Geo Pocket": known("ngp", "NG Pocket"),
  "SNK - Neo Geo Pocket Color": known("ngpc", "NG Pocket Color"),
  "FBNeo - Arcade Games": known("arcade", "Arcade"),
  "Atari - 2600": known("a2600", "Atari 2600"),
  "Atari - 5200": known("a5200", "Atari 5200"),
  "Atari - 7800": known("a7800", "Atari 7800"),
  "Atari - Lynx": known("lynx", "Lynx"),
  "Atari - Jaguar": known("jaguar", "Jaguar"),
  "Atari - ST": known("atarist", "Atari ST"),
  "Bandai - WonderSwan": known("ws", "WonderSwan"),
  "Bandai - WonderSwan Color": known("wsc", "WonderSwan Color"),
  "Coleco - ColecoVision": known("coleco", "ColecoVision"),
  "Mattel - Intellivision": known("intv", "Intellivision"),
  "GCE - Vectrex": known("vectrex", "Vectrex"),
  "Magnavox - Odyssey2": known("odyssey2", "Odyssey 2"),
  "Fairchild - Channel F": known("channelf", "Channel F"),
  "Watara - Supervision": known("supervision", "Supervision"),
  "The 3DO Company - 3DO": known("3do", "3DO"),
  "Microsoft - MSX": known("msx", "MSX"),
  "Microsoft - MSX2": known("msx", "MSX"),
  "Commodore - 64": known("c64", "C64"),
  "Commodore - Amiga": known("amiga", "Amiga"),
  "Commodore - VIC-20": known("vic20", "VIC-20"),
  "Sinclair - ZX Spectrum": known("zxspectrum", "ZX Spectrum"),
  "Sinclair - ZX 81": known("zx81", "ZX81"),
  "Amstrad - CPC": known("cpc", "Amstrad CPC"),
  "DOS": known("dos", "DOS"),
  "ScummVM": known("scummvm", "ScummVM"),
  "Sharp - X68000": known("x68000", "X68000"),
  "NEC - PC-98": known("pc98", "PC-98"),
};

/**
 * RetroArch's PlayStation and PSP games join the tabs of the games started
 * through Adrenaline: one tab per console.
 */
const SHARED: Record<string, string> = {
  "Sony - PlayStation": PSX_ID,
  "Sony - PlayStation Portable": PSP_ID,
};

/** The arcade databases: FBNeo's and every MAME version's (`MAME 2003-Plus`). */
const ARCADE = KNOWN["FBNeo - Arcade Games"];

const slug = (text: string) =>
  text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

/**
 * The category a console's games go to: its id, and the label of a category
 * this module makes (none for the built-in PSX and PSP). A console missing
 * from the table gets `retro-` and its name as a slug, and the part of its
 * name after the maker (`Nintendo - Virtual Boy` gives `Virtual Boy`).
 */
export function consoleCategory(database: string): Known & { builtin?: boolean } {
  const name = database.trim();
  const shared = SHARED[name];
  if (shared) return { id: shared, label: "", builtin: true };
  const listed = KNOWN[name];
  if (listed) return listed;
  if (name.startsWith("MAME")) return ARCADE;
  const parts = name.split(" - ");
  const label = (parts.length > 1 ? parts.slice(1).join(" - ") : name).slice(0, CATEGORY_LABEL_MAX).trim();
  return { id: `retro-${slug(name)}`.slice(0, 32).replace(/-+$/, ""), label: label || "RetroArch" };
}

/**
 * The categories RetroArch games need, one per id, in the order of their
 * first game; the built-in PSX and PSP are not among them.
 */
export function consoleCategories(consoles: Iterable<string>): Category[] {
  const result: Category[] = [];
  for (const name of consoles) {
    const category = consoleCategory(name);
    if (category.builtin || result.some((item) => item.id === category.id)) continue;
    result.push({ id: category.id, label: category.label, console: true });
  }
  return result;
}
