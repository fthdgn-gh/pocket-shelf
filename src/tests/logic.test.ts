// Unit tests for the launcher's host-independent logic: D-pad movement,
// category helpers, keyboard layout and title formatting. Run with
// `bun run shelf:test`.

import { describe, expect, test } from "bun:test";
import { cleanTitle } from "../catalog.ts";
import { categoryOf, cycleCategory, makeCategoryId } from "../categories.ts";
import { LETTER_ROWS, SYMBOL_ROWS, mapColumn } from "../keyboard.ts";
import { GRID_COLUMNS, GRID_ROWS, LIST_ROWS, SHELF, carouselLayout } from "../layout.ts";
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
