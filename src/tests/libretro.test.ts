// Unit tests for matching RetroArch games to libretro's thumbnails
// (`libretro.ts`). Run with `bun run shelf:test`.

import { describe, expect, test } from "bun:test";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { inflateSync } from "node:zlib";
import { scraperFor } from "../scrapers.ts";
import {
  baseName,
  createListing,
  libretroFolder,
  listingPath,
  matchName,
  regionsOf,
  scrubbed,
  searchNames,
  serverFolder,
  thumbnailUrl,
} from "../libretro.ts";

const LISTING = createListing([
  "Battleship (USA)",
  "Battleship (Europe)",
  "Frogger (USA)",
  "Frogger (Japan)",
  "Frogger (USA) (Beta)",
  "Adventures of Batman _ Robin, The (USA)",
  "Secret of Monkey Island, The (USA) (RE)",
  "Pokemon - Ruby Version (USA, Europe) (Rev 2)",
  "Stunt Race FX (USA)",
  "Stunt Race FX (Europe)",
  "Super Mario World (USA)",
  "Super Mario World 2 - Yoshi's Island (USA)",
  "Mario Paint (USA)",
]);

describe("names", () => {
  test("bases ignore regions, tags, articles, accents, & and punctuation", () => {
    expect(baseName("Battleship (U)")).toBe(baseName("Battleship (USA)"));
    expect(baseName("Frogger (UJ) [!]")).toBe("frogger");
    expect(baseName("Adventures of Batman & Robin, The (E)")).toBe(baseName("Adventures of Batman _ Robin, The (USA)"));
    expect(baseName("André Agassi Tennis (U)")).toBe("andreagassitennis");
  });

  test("regions come from either naming style", () => {
    expect(regionsOf("Battleship (U)")).toEqual(["USA"]);
    expect(regionsOf("Pele's World Tournament Soccer (JUE) [!]")).toEqual(["USA"]);
    expect(regionsOf("Ninja Gaiden (USA, Europe, Brazil) (En)")).toEqual(["USA", "Europe"]);
  });

  test("labels become file names as RetroArch scrubs them", () => {
    expect(scrubbed("Q*bert: The Arcade?")).toBe("Q_bert_ The Arcade_");
    expect(thumbnailUrl("Nintendo - Game Boy", "Named_Snaps", "Catrap (USA)")).toBe(
      "https://thumbnails.libretro.com/Nintendo%20-%20Game%20Boy/Named_Snaps/Catrap%20(USA).png",
    );
    expect(serverFolder("MAME 2003-Plus")).toBe("MAME");
    expect(serverFolder("Neo Geo")).toBe("SNK - Neo Geo");
    expect(listingPath("Nintendo - Super Nintendo Entertainment System")).toBe(
      "libretro/Nintendo-Super-Nintendo-Entertainment-System.txt",
    );
  });
});

describe("scraper choice", () => {
  const retro = { console: "Nintendo - Game Boy", platform: "retro" };
  const psx = { platform: "psx" };
  const homebrew = { platform: "vita" };
  test("Auto gives RetroArch games libretro and the rest SteamGridDB", () => {
    expect(scraperFor("auto", retro)).toBe("libretro");
    expect(scraperFor("auto", psx)).toBe("steamgriddb");
    expect(scraperFor("auto", homebrew)).toBe("steamgriddb");
  });
  test("a chosen scraper is used for every title it has pictures for", () => {
    expect(scraperFor("steamgriddb", retro)).toBe("steamgriddb");
    expect(scraperFor("libretro", psx)).toBe("libretro");
    expect(scraperFor("libretro", homebrew)).toBeNull();
  });
  test("PSX and PSP games have the PlayStation folders; Vita titles none", () => {
    expect(libretroFolder(psx)).toBe("Sony - PlayStation");
    expect(libretroFolder({ platform: "psp" })).toBe("Sony - PlayStation Portable");
    expect(libretroFolder(homebrew)).toBeUndefined();
    expect(libretroFolder({ console: "MAME 2003-Plus" })).toBe("MAME");
  });
});

describe("matchName", () => {
  test("uses the label itself when the server has it", () => {
    expect(matchName("Stunt Race FX (Europe)", LISTING)).toBe("Stunt Race FX (Europe)");
  });

  test("finds GoodTools and plain names, in the label's region or the preferred one", () => {
    expect(matchName("Battleship (U)", LISTING)).toBe("Battleship (USA)");
    expect(matchName("Battleship (E)", LISTING)).toBe("Battleship (Europe)");
    expect(matchName("Frogger (J) [!]", LISTING)).toBe("Frogger (Japan)");
    expect(matchName("Stunt Race FX", LISTING)).toBe("Stunt Race FX (USA)");
    expect(matchName("Adventures of Batman & Robin, The", LISTING)).toBe("Adventures of Batman _ Robin, The (USA)");
  });

  test("leaves out betas and unknown games", () => {
    expect(matchName("Frogger", LISTING)).toBe("Frogger (USA)");
    expect(matchName("Boogerman", LISTING)).toBeUndefined();
  });
});

describe("searchNames", () => {
  test("puts the match first, then names holding every word, shorter first", () => {
    expect(searchNames("mario world", LISTING)).toEqual([
      "Super Mario World (USA)",
      "Super Mario World 2 - Yoshi's Island (USA)",
    ]);
    expect(searchNames("Super Mario World", LISTING)[0]).toBe("Super Mario World (USA)");
    expect(searchNames("", LISTING)).toEqual([]);
  });
});

// With the user's playlists and the server's listings saved in .samples (not
// in git), how many games find a box art.
const SAMPLES = ".samples/retroarch-playlists";
const LISTINGS = ".samples/libretro-listings";
test.skipIf(!existsSync(SAMPLES) || !existsSync(LISTINGS))("the sample library finds most of its box art", () => {
  const rzip = (bytes: Buffer) => {
    const size = Number(bytes.readBigUInt64LE(12));
    const parts: Buffer[] = [];
    let at = 20;
    let total = 0;
    while (total < size) {
      const length = bytes.readUInt32LE(at);
      const part = inflateSync(bytes.subarray(at + 4, at + 4 + length));
      parts.push(part);
      total += part.length;
      at += 4 + length;
    }
    return Buffer.concat(parts).toString("utf8");
  };
  let games = 0;
  let found = 0;
  for (const file of readdirSync(SAMPLES).filter((name) => name.endsWith(".lpl"))) {
    const folder = serverFolder(file.slice(0, -4));
    const page = `${LISTINGS}/${folder.replace(/[ /]/g, "_")}.html`;
    if (!existsSync(page)) continue;
    const names = [...readFileSync(page, "utf8").matchAll(/href="([^"]+)\.png"/g)].map((match) =>
      decodeURIComponent(match[1]!),
    );
    const listing = createListing(names);
    const items = JSON.parse(rzip(readFileSync(`${SAMPLES}/${file}`))).items as { label: string }[];
    for (const item of items) {
      games++;
      if (matchName(item.label, listing)) found++;
    }
  }
  expect(games).toBeGreaterThan(5000);
  expect(found / games).toBeGreaterThan(0.68);
});
