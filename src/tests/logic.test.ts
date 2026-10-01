// Unit tests for the launcher's host-independent logic: D-pad movement,
// category helpers, keyboard layout and title formatting. Run with
// `bun run shelf:test`.

import { describe, expect, test } from "bun:test";
import { cleanTitle, shortTitle } from "../catalog.ts";
import { categoryOf, cycleCategory, makeCategoryId } from "../categories.ts";
import { LETTER_ROWS, SYMBOL_ROWS, mapColumn } from "../keyboard.ts";
import { GRID_COLUMNS, GRID_ROWS, LIST_ROWS, carouselLayout } from "../layout.ts";
import { iconRadius, moveSelection, pageSize } from "../navigation.ts";

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

  test("grid moves by one item sideways and by one row vertically", () => {
    expect(moveSelection("grid", "right", 3, 10)).toBe(4);
    expect(moveSelection("grid", "left", 4, 10)).toBe(3);
    expect(moveSelection("grid", "down", 1, 10)).toBe(1 + GRID_COLUMNS);
    expect(moveSelection("grid", "up", 5, 10)).toBe(5 - GRID_COLUMNS);
  });

  test("grid down onto a partial last row lands on the last item", () => {
    // 10 items: rows of 4, 4 and 2. Column 2 has nothing below row 1.
    expect(moveSelection("grid", "down", 6, 10)).toBe(9);
    // Already on the last row.
    expect(moveSelection("grid", "down", 8, 10)).toBeNull();
  });

  test("an empty list never yields a destination", () => {
    for (const view of ["carousel", "grid", "list"] as const) {
      for (const direction of ["left", "right", "up", "down"] as const) {
        expect(moveSelection(view, direction, 0, 0)).toBeNull();
      }
    }
  });
});

describe("paging and icon loading", () => {
  test("L and R jump one screen of items", () => {
    expect(pageSize("carousel")).toBe(1);
    expect(pageSize("grid")).toBe(GRID_COLUMNS * GRID_ROWS);
    expect(pageSize("list")).toBe(LIST_ROWS);
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
    expect(categoryOf("NPXS10001")).toBe("apps");
    expect(categoryOf("PBCF7609D")).toBe("homebrew");
  });

  test("cycleCategory wraps in both directions", () => {
    const list = ["games", "apps", "homebrew"];
    expect(cycleCategory(list, "games", 1)).toBe("apps");
    expect(cycleCategory(list, "homebrew", 1)).toBe("games");
    expect(cycleCategory(list, "games", -1)).toBe("homebrew");
  });

  test("cycleCategory tolerates an unknown current id and an empty list", () => {
    expect(cycleCategory(["games", "apps"], "gone", 1)).toBe("apps");
    expect(cycleCategory([], "games", 1)).toBe("games");
  });

  test("makeCategoryId slugs the label and avoids taken ids", () => {
    expect(makeCategoryId("Retro Games!", new Set())).toBe("retro-games");
    expect(makeCategoryId("Retro Games", new Set(["retro-games"]))).toBe("retro-games-2");
    expect(makeCategoryId("Retro Games", new Set(["retro-games", "retro-games-2"]))).toBe("retro-games-3");
    expect(makeCategoryId("!!!", new Set())).toBe("category");
  });
});

describe("keyboard", () => {
  test("every character key has a shifted twin", () => {
    for (const rows of [LETTER_ROWS, SYMBOL_ROWS]) {
      for (const row of rows) {
        for (const key of row) {
          expect(key.lower.length).toBeGreaterThan(0);
          expect(key.upper?.length ?? 0).toBeGreaterThan(0);
        }
      }
    }
  });

  test("both layouts have the same number of rows and share the bottom row", () => {
    expect(SYMBOL_ROWS.length).toBe(LETTER_ROWS.length);
    expect(SYMBOL_ROWS.at(-1)).toBe(LETTER_ROWS.at(-1));
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

  test("mapColumn always lands on an existing key", () => {
    for (const rows of [LETTER_ROWS, SYMBOL_ROWS]) {
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
  });

  test("shortTitle cuts to the limit, ellipsis included", () => {
    expect(shortTitle("short", 8)).toBe("short");
    expect(shortTitle("abcdefghij", 8)).toBe("abcde...");
    expect(shortTitle("abcd efghij", 8)).toBe("abcd...");
  });
});

describe("layout", () => {
  test("carouselLayout centers the first card in the viewport", () => {
    expect(carouselLayout(480)).toEqual({ cardPitch: 166, centerX: 165 });
    expect(carouselLayout(960).centerX).toBe(405);
  });
});
