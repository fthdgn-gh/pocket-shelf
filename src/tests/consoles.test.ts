// Unit tests for the categories of RetroArch games (`consoles.ts`). Run with
// `bun run shelf:test`.

import { describe, expect, test } from "bun:test";
import { CATEGORY_ID_PATTERN, CATEGORY_LABEL_MAX, PSP_ID, PSX_ID } from "../categories.ts";
import { consoleCategories, consoleCategory } from "../consoles.ts";

describe("consoleCategory", () => {
  test("gives known consoles a short label and a fixed id", () => {
    expect(consoleCategory("Nintendo - Super Nintendo Entertainment System")).toEqual({ id: "retro-snes", label: "SNES" });
    expect(consoleCategory("Sega - Mega Drive - Genesis")).toEqual({ id: "retro-md", label: "Mega Drive" });
  });

  test("puts every arcade database in one category", () => {
    const arcade = consoleCategory("FBNeo - Arcade Games").id;
    expect(consoleCategory("MAME 2003-Plus").id).toBe(arcade);
    expect(consoleCategory("MAME 2000").id).toBe(arcade);
  });

  test("sends RetroArch's PlayStation and PSP games to the built-in tabs", () => {
    expect(consoleCategory("Sony - PlayStation")).toMatchObject({ id: PSX_ID, builtin: true });
    expect(consoleCategory("Sony - PlayStation Portable")).toMatchObject({ id: PSP_ID, builtin: true });
  });

  test("names an unknown console after the part following its maker", () => {
    const category = consoleCategory("Acme - Super Handheld Deluxe Edition Plus");
    expect(category.label).toBe("Super Handheld D");
    expect(category.label.length).toBeLessThanOrEqual(CATEGORY_LABEL_MAX);
    expect(category.id).toMatch(CATEGORY_ID_PATTERN);
    expect(category.id.startsWith("retro-acme-super")).toBe(true);
    expect(consoleCategory("Homebrew Things")).toEqual({ id: "retro-homebrew-things", label: "Homebrew Things" });
  });

  test("every known id is a valid category id", () => {
    for (const name of ["Nintendo - Game Boy", "Bandai - WonderSwan Color", "SNK - Neo Geo Pocket Color"]) {
      const category = consoleCategory(name);
      expect(category.id).toMatch(CATEGORY_ID_PATTERN);
      expect(category.label.length).toBeLessThanOrEqual(CATEGORY_LABEL_MAX);
    }
  });
});

describe("consoleCategories", () => {
  test("lists each category once, in the order of its first game, without the built-ins", () => {
    const list = consoleCategories([
      "Nintendo - Game Boy",
      "MAME 2003-Plus",
      "Sony - PlayStation",
      "Nintendo - Game Boy",
      "FBNeo - Arcade Games",
    ]);
    expect(list.map((item) => item.id)).toEqual(["retro-gb", "retro-arcade"]);
    expect(list.every((item) => item.console === true)).toBe(true);
  });
});
