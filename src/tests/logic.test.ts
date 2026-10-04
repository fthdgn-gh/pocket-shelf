// Unit tests for the launcher's host-independent logic: D-pad movement,
// category helpers, keyboard layout and title formatting. Run with
// `bun run shelf:test`.

import { describe, expect, test } from "bun:test";
import { cleanTitle, isListed } from "../catalog.ts";
import { BUILTIN_CATEGORIES, PSM_ID, PSP_ID, PSX_ID, SYSTEM_ID, categoryOf, cycleCategory, isPsmId, makeCategoryId } from "../categories.ts";
import { LANGUAGES, MESSAGES, fill, upper } from "../i18n.ts";
import { ACCENTS_FIRST, LETTER_ROWS, SYMBOL_ROWS, accentRows, mapColumn } from "../keyboard.ts";
import { GRID_COLUMNS, GRID_ROWS, LIST_ROWS, LIST_ROWS_UNDER_STATUS, SHELF, carouselLayout } from "../layout.ts";
import { MENU, menuPath, parentPage, type MenuPage } from "../menu-items.ts";
import { iconRadius, moveSelection, pageSize } from "../navigation.ts";
import { filesInUse, withoutFiles } from "../overrides.ts";
import { RECENT_MAX, pushRecent } from "../recent.ts";
import { missingArt } from "../scrape.ts";
import { searchTitles } from "../search.ts";

describe("moveSelection", () => {
  test("carousel moves on the horizontal axis only", () => {
    expect(moveSelection("carousel", "right", 0, 3)).toBe(1);
    expect(moveSelection("carousel", "left", 1, 3)).toBe(0);
    expect(moveSelection("carousel", "up", 1, 3)).toBeNull();
    expect(moveSelection("carousel", "down", 1, 3)).toBeNull();
  });

  test("list moves on the vertical axis only", () => {
    expect(moveSelection("list", "down", 0, 3)).toBe(1);
    expect(moveSelection("list", "up", 1, 3)).toBe(0);
    expect(moveSelection("list", "left", 1, 3)).toBeNull();
    expect(moveSelection("list", "right", 1, 3)).toBeNull();
  });

  test("presses at either end of the list go nowhere", () => {
    expect(moveSelection("carousel", "left", 0, 3)).toBeNull();
    expect(moveSelection("carousel", "right", 2, 3)).toBeNull();
    expect(moveSelection("list", "up", 0, 3)).toBeNull();
    expect(moveSelection("list", "down", 2, 3)).toBeNull();
    expect(moveSelection("grid", "left", 0, 10)).toBeNull();
    expect(moveSelection("grid", "up", 2, 10)).toBeNull();
  });

  // Two full rows and a last row of two items.
  const GRID_COUNT = GRID_COLUMNS * 2 + 2;

  test("grid moves by one item sideways and by one row vertically", () => {
    // Right from the end of a row continues onto the next one.
    expect(moveSelection("grid", "right", GRID_COLUMNS - 1, GRID_COUNT)).toBe(GRID_COLUMNS);
    expect(moveSelection("grid", "left", GRID_COLUMNS, GRID_COUNT)).toBe(GRID_COLUMNS - 1);
    expect(moveSelection("grid", "down", 1, GRID_COUNT)).toBe(1 + GRID_COLUMNS);
    expect(moveSelection("grid", "up", 1 + GRID_COLUMNS, GRID_COUNT)).toBe(1);
  });

  test("grid down onto a partial last row lands on the last item", () => {
    // The last column of the second row has nothing below it.
    expect(moveSelection("grid", "down", GRID_COLUMNS * 2 - 1, GRID_COUNT)).toBe(GRID_COUNT - 1);
    // Already on the last row.
    expect(moveSelection("grid", "down", GRID_COLUMNS * 2, GRID_COUNT)).toBeNull();
  });

  test("an empty list never yields a destination", () => {
    for (const view of ["carousel", "grid", "list"] as const) {
      for (const direction of ["left", "right", "up", "down"] as const) {
        expect(moveSelection(view, direction, 0, 0)).toBeNull();
        expect(moveSelection(view, direction, 0, 0, true)).toBeNull();
      }
    }
  });

  test("with wrap, a press past an end continues at the other end", () => {
    expect(moveSelection("carousel", "right", 2, 3, true)).toBe(0);
    expect(moveSelection("carousel", "left", 0, 3, true)).toBe(2);
    expect(moveSelection("list", "down", 2, 3, true)).toBe(0);
    expect(moveSelection("list", "up", 0, 3, true)).toBe(2);
    expect(moveSelection("grid", "right", GRID_COUNT - 1, GRID_COUNT, true)).toBe(0);
    expect(moveSelection("grid", "left", 0, GRID_COUNT, true)).toBe(GRID_COUNT - 1);
    // Inside the list, wrap changes nothing.
    expect(moveSelection("carousel", "right", 0, 3, true)).toBe(1);
    // The axis a view does not use stays unused.
    expect(moveSelection("carousel", "up", 0, 3, true)).toBeNull();
    expect(moveSelection("list", "left", 0, 3, true)).toBeNull();
  });

  test("with wrap, the grid keeps the column between its first and last rows", () => {
    // Column 1 exists in the last row (two items).
    expect(moveSelection("grid", "up", 1, GRID_COUNT, true)).toBe(GRID_COLUMNS * 2 + 1);
    expect(moveSelection("grid", "down", GRID_COLUMNS * 2 + 1, GRID_COUNT, true)).toBe(1);
    // Column 3 does not: up from the first row lands on the last item.
    expect(moveSelection("grid", "up", 3, GRID_COUNT, true)).toBe(GRID_COUNT - 1);
    // A single row has no other row to go to.
    expect(moveSelection("grid", "up", 1, 3, true)).toBeNull();
    expect(moveSelection("grid", "down", 1, 3, true)).toBeNull();
  });

  test("with wrap, a list of one item has nowhere to go", () => {
    expect(moveSelection("carousel", "right", 0, 1, true)).toBeNull();
    expect(moveSelection("list", "up", 0, 1, true)).toBeNull();
  });
});

describe("paging and icon loading", () => {
  test("L and R jump one screen of items", () => {
    expect(pageSize("carousel")).toBe(1);
    expect(pageSize("grid")).toBe(GRID_COLUMNS * GRID_ROWS);
    expect(pageSize("list")).toBe(LIST_ROWS);
    expect(pageSize("list", true)).toBe(LIST_ROWS_UNDER_STATUS);
  });

  test("icons load for at least the visible items around the selection", () => {
    expect(iconRadius("grid")).toBeGreaterThanOrEqual(GRID_COLUMNS * GRID_ROWS);
    expect(iconRadius("list")).toBeGreaterThanOrEqual(LIST_ROWS);
    expect(iconRadius("carousel")).toBeGreaterThanOrEqual(1);
  });
});

describe("categories", () => {
  test("categoryOf reads the title id prefix", () => {
    expect(categoryOf("PCSA00069")).toBe("games");
    expect(categoryOf("NPXS10001")).toBe("system");
    expect(categoryOf("PBCF7609D")).toBe("homebrew");
    expect(categoryOf("NPOA00013")).toBe(PSM_ID);
    expect(categoryOf("NPNA00042")).toBe(PSM_ID);
    // PSP and PS1 (PSX) games by the platform the host reports, whatever the id.
    expect(categoryOf("NPUZ00001", "psp")).toBe(PSP_ID);
    expect(categoryOf("SCUS94163", "psx")).toBe(PSX_ID);
    expect(categoryOf("SCUS94163")).toBe("homebrew");
  });

  test("the default tab order ends with System; PS1 games are the PSX tab", () => {
    expect(BUILTIN_CATEGORIES.map((item) => item.id)).toEqual(["games", "homebrew", PSM_ID, PSP_ID, PSX_ID, SYSTEM_ID]);
    expect(PSX_ID).toBe("builtin-psx");
    for (const { id } of LANGUAGES) expect(MESSAGES[id].categories.psx).toBe("PSX");
  });

  test("cycleCategory wraps in both directions", () => {
    const list = ["games", "system", "homebrew"];
    expect(cycleCategory(list, "games", 1)).toBe("system");
    expect(cycleCategory(list, "homebrew", 1)).toBe("games");
    expect(cycleCategory(list, "games", -1)).toBe("homebrew");
  });

  test("cycleCategory tolerates an unknown current id and an empty list", () => {
    expect(cycleCategory(["games", "system"], "gone", 1)).toBe("system");
    expect(cycleCategory([], "games", 1)).toBe("games");
  });

  test("makeCategoryId slugs the label and avoids taken ids", () => {
    expect(makeCategoryId("Retro Games!", new Set())).toBe("retro-games");
    expect(makeCategoryId("Retro Games", new Set(["retro-games"]))).toBe("retro-games-2");
    expect(makeCategoryId("Retro Games", new Set(["retro-games", "retro-games-2"]))).toBe("retro-games-3");
    expect(makeCategoryId("!!!", new Set())).toBe("category");
  });
});

describe("last played", () => {
  test("pushRecent puts the started title first and lists it once", () => {
    expect(pushRecent([], "PCSA00001")).toEqual(["PCSA00001"]);
    expect(pushRecent(["PCSA00001", "PCSA00002"], "PCSA00003")).toEqual(["PCSA00003", "PCSA00001", "PCSA00002"]);
    expect(pushRecent(["PCSA00001", "PCSA00002", "PCSA00003"], "PCSA00003")).toEqual([
      "PCSA00003",
      "PCSA00001",
      "PCSA00002",
    ]);
  });

  test("pushRecent drops the oldest title past the limit", () => {
    const full = Array.from({ length: RECENT_MAX }, (_, index) => `PCSA${String(index).padStart(5, "0")}`);
    const next = pushRecent(full, "PCSB00000");
    expect(next.length).toBe(RECENT_MAX);
    expect(next[0]).toBe("PCSB00000");
    expect(next).not.toContain(full[RECENT_MAX - 1]);
  });
});

describe("search", () => {
  const games = [
    { id: "PCSE00317", title: "Castle Siege Tactics" },
    { id: "PCSE00444", title: "Iron Fist Arena" },
    { id: "PCSB00800", title: "Pokémon Stadium" },
    { id: "PCSF00042", title: "Star Harbor" },
    { id: "VITASHELL", title: "VitaShell" },
  ];
  const ids = (term: string) => searchTitles(games, term).map((game) => game.id);

  test("an empty term finds nothing", () => {
    expect(ids("")).toEqual([]);
    expect(ids("   ")).toEqual([]);
  });

  test("titles that start with the term come first, then word starts, then the rest", () => {
    // "Star Harbor" starts with it, "Pokémon Stadium" has a word that does,
    // and the other two have it inside a word.
    expect(ids("st")).toEqual(["PCSF00042", "PCSB00800", "PCSE00317", "PCSE00444"]);
  });

  test("case and the marks on letters are ignored", () => {
    expect(ids("POKEMON")).toEqual(["PCSB00800"]);
    expect(ids("pokémon")).toEqual(["PCSB00800"]);
  });

  test("letters without a base letter match their plain spelling", () => {
    const titles = [
      { id: "PCSB00001", title: "Işık Kılıcı" },
      { id: "PCSB00002", title: "Straße" },
      { id: "PCSB00003", title: "Œuvre" },
      { id: "PCSB00004", title: "İstanbul" },
    ];
    const find = (term: string) => searchTitles(titles, term).map((game) => game.id);
    expect(find("isik kilici")).toEqual(["PCSB00001"]);
    expect(find("strasse")).toEqual(["PCSB00002"]);
    expect(find("oeuvre")).toEqual(["PCSB00003"]);
    expect(find("istanbul")).toEqual(["PCSB00004"]);
  });

  test("every word has to match, in any order", () => {
    expect(ids("tactics castle")).toEqual(["PCSE00317"]);
    expect(ids("castle arena")).toEqual([]);
  });

  test("a title id matches too", () => {
    expect(ids("pcsf")).toEqual(["PCSF00042"]);
  });
});

describe("fetching artwork for many titles", () => {
  const game = { title: "Neon Drift", id: "PCSB00245", genre: "PS Vita", category: "games", tint: 0 };

  test("a title with no files of its own lacks both pictures", () => {
    expect(missingArt(game)).toEqual(["icon", "backdrop"]);
  });

  test("an icon the user chose, one matched by name, and the title's own icon are kept", () => {
    expect(missingArt({ ...game, art: "neon.png" })).toEqual(["backdrop"]);
    expect(missingArt({ ...game, art: "PCSB00245.png", artAuto: true })).toEqual(["backdrop"]);
    expect(missingArt({ ...game, artIcon: true })).toEqual(["backdrop"]);
  });

  test("a chosen backdrop, or the choice of none, is kept", () => {
    expect(missingArt({ ...game, backdrop: "neon.png" })).toEqual(["icon"]);
    expect(missingArt({ ...game, backdrop: "none" })).toEqual(["icon"]);
    expect(missingArt({ ...game, art: "neon.png", backdrop: "neon.png" })).toEqual([]);
  });
});

describe("cleaning up artwork", () => {
  const overrides = {
    PCSA00069: { art: "PCSA00069-1.png", backdrop: "PCSA00069-2.png", favorite: true as const },
    PCSB00245: { art: "icon", backdrop: "none", title: "Neon" },
    PCSE00317: { backdrop: "PCSE00317-3.png" },
  };

  test("filesInUse lists the files the titles name, not the choices without a file", () => {
    expect([...filesInUse(overrides, "art")]).toEqual(["PCSA00069-1.png"]);
    expect([...filesInUse(overrides, "backdrop")].sort()).toEqual(["PCSA00069-2.png", "PCSE00317-3.png"]);
  });

  test("withoutFiles drops the files and keeps every other change", () => {
    expect(withoutFiles(overrides)).toEqual({
      PCSA00069: { favorite: true },
      PCSB00245: { art: "icon", backdrop: "none", title: "Neon" },
    });
  });
});

describe("languages", () => {
  // The `{name}` places of every text, by key, nested texts included.
  const places = (messages: object, prefix = ""): Record<string, string[]> => {
    const found: Record<string, string[]> = {};
    for (const [key, value] of Object.entries(messages)) {
      if (typeof value === "string") found[prefix + key] = (value.match(/\{\w+\}/g) ?? []).sort();
      else if (Array.isArray(value)) found[prefix + key] = [];
      else Object.assign(found, places(value, `${prefix}${key}.`));
    }
    return found;
  };

  test("every language has the same texts with the same places as English", () => {
    for (const { id } of LANGUAGES) {
      expect(places(MESSAGES[id])).toEqual(places(MESSAGES.en));
    }
  });

  test("no text is left empty", () => {
    const texts = (value: unknown): string[] =>
      typeof value === "string" ? [value] : Object.values(value as object).flatMap(texts);
    for (const { id } of LANGUAGES) {
      for (const text of texts(MESSAGES[id])) expect(text.trim().length).toBeGreaterThan(0);
    }
  });

  test("fill replaces the places it has a value for", () => {
    expect(fill("Launching {title}...", { title: "Gravity Daze" })).toBe("Launching Gravity Daze...");
    expect(fill("{count} found", { count: 3 })).toBe("3 found");
    expect(fill("Press {button} again", {})).toBe("Press {button} again");
  });

  test("upper keeps the Turkish dotted and dotless i apart", () => {
    expect(upper("Kategoriler", "tr")).toBe("KATEGORİLER");
    expect(upper("Yazı tipi", "tr")).toBe("YAZI TİPİ");
    expect(upper("Kategorien", "de")).toBe("KATEGORIEN");
  });
});

describe("keyboard", () => {
  const PAGES = [LETTER_ROWS, SYMBOL_ROWS, ...LANGUAGES.map(({ id }) => accentRows(id))];

  test("every character key has a shifted twin", () => {
    for (const rows of PAGES) {
      for (const row of rows) {
        for (const key of row) {
          expect(key.lower.length).toBeGreaterThan(0);
          expect(key.upper?.length ?? 0).toBeGreaterThan(0);
        }
      }
    }
  });

  test("every page has the same number of rows and shares the bottom row", () => {
    for (const rows of PAGES) {
      expect(rows.length).toBe(LETTER_ROWS.length);
      expect(rows.at(-1)).toBe(LETTER_ROWS.at(-1));
    }
    expect(LETTER_ROWS.at(-1)?.map((key) => key.action)).toEqual([
      "shift",
      "symbols",
      "space",
      "delete",
      "done",
    ]);
  });

  test("mapColumn keeps the highlight under the same part of the row", () => {
    expect(mapColumn(10, 5, 0)).toBe(0);
    expect(mapColumn(10, 5, 9)).toBe(4);
    expect(mapColumn(5, 10, 2)).toBe(5);
    expect(mapColumn(10, 2, 9)).toBe(1);
  });

  test("the accents page has the same thirty letters in every language, its own first", () => {
    const letters = (language: string) =>
      accentRows(language)
        .slice(1, 4)
        .flat()
        .map((key) => key.lower);
    const all = [...letters("en")].sort();
    expect(all.length).toBe(30);
    expect(new Set(all).size).toBe(30);
    for (const { id } of LANGUAGES) {
      expect([...letters(id)].sort()).toEqual(all);
      const first = [...(ACCENTS_FIRST[id] ?? "")];
      expect(letters(id).slice(0, first.length)).toEqual(first);
    }
  });

  test("the Turkish page pairs each i with its own capital", () => {
    const keys = accentRows("tr").flat();
    expect(keys.find((key) => key.lower === "ı")?.upper).toBe("I");
    expect(keys.find((key) => key.lower === "i")?.upper).toBe("İ");
  });

  test("mapColumn always lands on an existing key", () => {
    for (const rows of PAGES) {
      for (const from of rows) {
        for (const to of rows) {
          for (let col = 0; col < from.length; col++) {
            const mapped = mapColumn(from.length, to.length, col);
            expect(mapped).toBeGreaterThanOrEqual(0);
            expect(mapped).toBeLessThan(to.length);
          }
        }
      }
    }
  });
});

describe("titles", () => {
  test("cleanTitle drops trademark glyphs and collapses spaces", () => {
    expect(cleanTitle("Uncharted™:  Golden Abyss®")).toBe("Uncharted: Golden Abyss");
    expect(cleanTitle("  Plain  ")).toBe("Plain");
    expect(cleanTitle("PlayStation®Store")).toBe("PlayStation Store");
  });

  test("Adrenaline's game bubbles are not listed; Adrenaline and other titles are", () => {
    expect(isListed("PSPEMU001")).toBe(false);
    expect(isListed("PSPEMU123")).toBe(false);
    expect(isListed("PSPEMUCFW")).toBe(true);
    expect(isListed("PCSA00069")).toBe(true);
    expect(isListed("VITASHELL")).toBe(true);
  });

  test("system applications with a home screen bubble are listed; services are not", () => {
    expect(isListed("NPXS10015")).toBe(true); // Settings
    expect(isListed("NPXS10003")).toBe(true); // Internet Browser
    expect(isListed("NPXS10016")).toBe(false); // the Settings dialog
    expect(isListed("NPXS10079")).toBe(false); // Daily Checker BG
  });
});

describe("layout", () => {
  test("carouselLayout centers the first tile for every detail level", () => {
    for (const detail of ["basic", "normal", "detailed"] as const) {
      const { tile, gap } = SHELF[detail];
      expect(carouselLayout(480, detail)).toEqual({ cardPitch: tile + gap, centerX: (480 - tile) / 2 });
    }
  });

  test("the selected shelf tile clears its neighbors and fits its row", () => {
    for (const { tile, gap, row } of Object.values(SHELF)) {
      // Selected: 125% plus a 3 px frame. Neighbors: 90%.
      const selectedHalf = (tile * 1.25) / 2 + 3 * 1.25;
      const neighborHalf = (tile * 0.9) / 2;
      expect(tile + gap - selectedHalf - neighborHalf).toBeGreaterThan(0);
      // The row has 8 px of padding under the tiles for the frame.
      expect(tile * 1.25 + 3 * 1.25).toBeLessThanOrEqual(row - 8);
    }
  });
});

describe("SELECT menu", () => {
  test("every row is on exactly one page, and every page is reached from the main page", () => {
    const pages = Object.keys(MENU) as MenuPage[];
    const rows = pages.flatMap((page) => MENU[page]);
    expect(new Set(rows).size).toBe(rows.length);
    for (const page of pages) if (page !== "main") expect(menuPath(page)[0]).toBe("main");
  });

  test("Status bar sits inside Appearance", () => {
    expect(parentPage("status")).toBe("appearance");
    expect(menuPath("clock")).toEqual(["main", "appearance", "status"]);
  });

  test("language is two presses up from the first row of the main page", () => {
    expect(MENU.main.at(-2)).toBe("language");
  });
});

test("isPsmId", () => {
  for (const id of ["NPNA00001", "NPOA00013", "NPPA00007", "NPQA12345"]) expect(isPsmId(id)).toBe(true);
  for (const id of ["NPXS10001", "NPEA00001", "NPOA0001", "NPOA000130", "NPOAX0013", "PCSE00001"]) {
    expect(isPsmId(id)).toBe(false);
  }
});
