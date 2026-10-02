// Unit tests for the SteamGridDB module: the URLs it asks for and how it reads
// the replies. Run with `bun run shelf:test`.

import { describe, expect, test } from "bun:test";
import {
  ASSETS_MAX,
  GAMES_MAX,
  assetFile,
  assetPath,
  fileBelongsTo,
  fileSlug,
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
    const owner = { id: "PCSA00069" };
    expect(assetFile(asset, owner)).toBe("PCSA00069-9001.png");
    expect(assetPath("icon", asset, owner)).toBe("art/PCSA00069-9001.png");
    expect(assetPath("backdrop", asset, owner)).toBe("backdrops/PCSA00069-9001.png");
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

describe("file names", () => {
  test("fileSlug keeps letters and digits, with a dash between words", () => {
    expect(fileSlug("Pokémon: Stadium 2")).toBe("pokemon-stadium-2");
    expect(fileSlug("  Uncharted™ -- Golden Abyss  ")).toBe("uncharted-golden-abyss");
    expect(fileSlug("Işık Kılıcı")).toBe("isik-kilici");
    expect(fileSlug("!!!")).toBe("");
  });

  test("fileBelongsTo matches a file named after the title or its id", () => {
    const names = ["Gravity Daze", "PCSA00069"];
    expect(fileBelongsTo("PCSA00069-48213.png", names)).toBe(true);
    expect(fileBelongsTo("gravity-daze-48213.png", names)).toBe(true);
    expect(fileBelongsTo("Gravity Daze.png", names)).toBe(true);
    expect(fileBelongsTo("PCSA00069.png", names)).toBe(true);
    expect(fileBelongsTo("gravity-rush-48213.png", names)).toBe(false);
    // The name has to end at a word: "gravity-dazed" is another title.
    expect(fileBelongsTo("gravity-dazed-1.png", names)).toBe(false);
    expect(fileBelongsTo("PCSA000690-1.png", names)).toBe(false);
    expect(fileBelongsTo("anything.png", ["", "!!!"])).toBe(false);
  });
});

describe("failed replies", () => {
  test("a rejected key, a busy server and an unknown status are each named", () => {
    expect(parseGames(401, '{"success":false,"errors":["Authentication Required"]}')).toEqual({
      problem: "keyRejected",
      status: 401,
    });
    expect(parseAssets(429, "")).toEqual({ problem: "serverBusy", status: 429 });
    expect(parseAssets(503, "")).toEqual({ problem: "serverStatus", status: 503 });
  });

  test("a reply that is not the expected JSON is named", () => {
    for (const text of ["", "<html>", '{"success":false}', '{"success":true,"data":{}}']) {
      expect(parseGames(200, text)).toEqual({ problem: "badReply", status: 200 });
    }
  });
});
