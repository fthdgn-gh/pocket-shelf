// Unit tests for the SteamGridDB module: the URLs it asks for and how it reads
// the replies. Run with `bun run shelf:test`.

import { describe, expect, test } from "bun:test";
import {
  ASSETS_MAX,
  GAMES_MAX,
  assetFile,
  assetPath,
  assetsUrl,
  authorization,
  parseAssets,
  parseGames,
  searchUrl,
} from "../steamgriddb.ts";

const reply = (data: unknown) => JSON.stringify({ success: true, data });

describe("requests", () => {
  test("the search term is trimmed and escaped into the path", () => {
    expect(searchUrl("  Gravity Rush  ")).toBe(
      "https://www.steamgriddb.com/api/v2/search/autocomplete/Gravity%20Rush",
    );
    expect(searchUrl("Persona 4: Golden / Vita?")).toBe(
      "https://www.steamgriddb.com/api/v2/search/autocomplete/Persona%204%3A%20Golden%20%2F%20Vita%3F",
    );
  });

  test("asset lists ask for static PNG files in sizes the decoders accept", () => {
    expect(assetsUrl("icon", 42)).toBe(
      "https://www.steamgriddb.com/api/v2/icons/game/42?types=static&mimes=image/png&dimensions=128,256,512",
    );
    expect(assetsUrl("backdrop", 42)).toBe(
      "https://www.steamgriddb.com/api/v2/heroes/game/42?types=static&mimes=image/png&dimensions=1920x620,1600x650",
    );
  });

  test("the key is sent as a bearer token", () => {
    expect(authorization("abc123")).toBe("Bearer abc123");
  });

  test("a candidate is saved under its id, in the folder for its kind", () => {
    const asset = { id: 9001, url: "https://cdn2.steamgriddb.com/hero/x.png" };
    expect(assetFile(asset)).toBe("sgdb-9001.png");
    expect(assetPath("icon", asset)).toBe("art/sgdb-9001.png");
    expect(assetPath("backdrop", asset)).toBe("backdrops/sgdb-9001.png");
  });
});

describe("parseGames", () => {
  test("reads id, name and release year", () => {
    const text = reply([
      { id: 1, name: "Gravity Rush", release_date: 1339459200, types: ["steam"] },
      { id: 2, name: "No Date" },
      { id: 3, name: "Zero Date", release_date: 0 },
    ]);
    expect(parseGames(200, text)).toEqual([
      { id: 1, name: "Gravity Rush", year: 2012 },
      { id: 2, name: "No Date", year: undefined },
      { id: 3, name: "Zero Date", year: undefined },
    ]);
  });

  test("skips entries without an id or a name, and caps the list", () => {
    expect(parseGames(200, reply([{ name: "No id" }, { id: 5 }, { id: 6, name: "" }, null]))).toEqual([]);
    const many = Array.from({ length: GAMES_MAX + 8 }, (_, index) => ({ id: index + 1, name: `Game ${index}` }));
    expect(parseGames(200, reply(many))).toHaveLength(GAMES_MAX);
  });

  test("no match is an empty list, not an error", () => {
    expect(parseGames(200, reply([]))).toEqual([]);
    expect(parseGames(404, "")).toEqual([]);
  });
});

describe("parseAssets", () => {
  test("keeps https PNG files and caps the list", () => {
    const text = reply([
      { id: 10, url: "https://cdn2.steamgriddb.com/icon/a.png", thumb: "https://cdn2.steamgriddb.com/thumb/a.jpg" },
      { id: 11, url: "https://cdn2.steamgriddb.com/icon/b.PNG" },
      { id: 12, url: "https://cdn2.steamgriddb.com/icon/c.ico" },
      { id: 13, url: "https://cdn2.steamgriddb.com/hero/d.jpg" },
      { id: 14, url: "http://cdn2.steamgriddb.com/icon/e.png" },
      { url: "https://cdn2.steamgriddb.com/icon/f.png" },
    ]);
    expect(parseAssets(200, text)).toEqual([
      { id: 10, url: "https://cdn2.steamgriddb.com/icon/a.png" },
      { id: 11, url: "https://cdn2.steamgriddb.com/icon/b.PNG" },
    ]);
    const many = Array.from({ length: ASSETS_MAX + 5 }, (_, index) => ({
      id: index + 1,
      url: `https://cdn2.steamgriddb.com/icon/${index}.png`,
    }));
    expect(parseAssets(200, reply(many))).toHaveLength(ASSETS_MAX);
  });
});

describe("failed replies", () => {
  test("a rejected key, a busy server and an unknown status each get a message", () => {
    expect(parseGames(401, '{"success":false,"errors":["Authentication Required"]}')).toBe(
      "SteamGridDB did not accept the API key.",
    );
    expect(parseAssets(429, "")).toBe("SteamGridDB is busy. Try again in a moment.");
    expect(parseAssets(503, "")).toBe("SteamGridDB answered with status 503.");
  });

  test("a reply that is not the expected JSON gets a message", () => {
    for (const text of ["", "<html>", '{"success":false}', '{"success":true,"data":{}}']) {
      expect(parseGames(200, text)).toBe("SteamGridDB sent an unexpected reply.");
    }
  });
});
