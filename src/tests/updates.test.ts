import { describe, expect, test } from "bun:test";
import {
  checkDue,
  compareVersions,
  CHECK_INTERVAL_MS,
  parseReleases,
  pickUpdate,
  type BuildInfo,
  type Release,
} from "../updates.ts";

const NIGHTLY_COMMIT = "a".repeat(40);
const OTHER_COMMIT = "b".repeat(40);

const release = (tag: string, extra: Record<string, unknown> = {}) => ({
  tag_name: tag,
  name: `Pocket Shelf ${tag}`,
  draft: false,
  prerelease: tag !== "shelf-v0.1.0",
  target_commitish: "main",
  published_at: "2026-10-05T19:35:55Z",
  assets: [
    { name: "pocket-shelf.vpk", browser_download_url: `https://github.com/x/releases/download/${tag}/pocket-shelf.vpk`, size: 2450654 },
  ],
  ...extra,
});

const REPLY = JSON.stringify([
  release("shelf-nightly", { target_commitish: NIGHTLY_COMMIT }),
  release("shelf-v0.2.0-beta.1"),
  release("shelf-v0.1.0"),
  release("shelf-v0.1.0-beta.2"),
  release("shelf-v0.1.0-beta.1"),
  release("shelf-v0.3.0", { draft: true }),
  release("shelf-v0.2.5", { assets: [] }),
  release("v0.14.0"),
]);

const build = (version: string, channel: BuildInfo["channel"], commit = OTHER_COMMIT): BuildInfo => ({
  version,
  channel,
  commit,
});

describe("parseReleases", () => {
  test("keeps published shelf releases that carry the VPK", () => {
    const tags = parseReleases(REPLY)!.map((item) => item.tag);
    expect(tags).toEqual(["shelf-nightly", "shelf-v0.2.0-beta.1", "shelf-v0.1.0", "shelf-v0.1.0-beta.2", "shelf-v0.1.0-beta.1"]);
  });

  test("reads the version from the tag and the nightly's commit", () => {
    const [nightly, beta] = parseReleases(REPLY)!;
    expect(nightly!.version).toBeUndefined();
    expect(nightly!.commit).toBe(NIGHTLY_COMMIT);
    expect(beta!.version).toBe("0.2.0-beta.1");
    expect(beta!.date).toBe("2026-10-05");
    expect(beta!.size).toBe(2450654);
  });

  test("a nightly without a commit id is left out", () => {
    expect(parseReleases(JSON.stringify([release("shelf-nightly")]))).toEqual([]);
  });

  test("a reply that is not the list is null", () => {
    expect(parseReleases('{"message":"API rate limit exceeded"}')).toBeNull();
    expect(parseReleases("<html>")).toBeNull();
  });
});

describe("compareVersions", () => {
  test("orders betas before their release", () => {
    const sorted = ["0.1.0", "0.2.0-beta.1", "0.1.0-beta.10", "0.1.0-beta.2", "0.10.0"].sort(compareVersions);
    expect(sorted).toEqual(["0.1.0-beta.2", "0.1.0-beta.10", "0.1.0", "0.2.0-beta.1", "0.10.0"]);
  });

  test("equal versions compare as 0", () => {
    expect(compareVersions("0.1.0-beta.1", "0.1.0-beta.1")).toBe(0);
  });
});

describe("pickUpdate", () => {
  const releases: Release[] = parseReleases(REPLY)!;
  const tagOf = (picked: Release | undefined) => picked?.tag;

  test("stable offers the newest release without a beta number", () => {
    expect(tagOf(pickUpdate(releases, "stable", build("0.1.0-beta.1", "beta")))).toBe("shelf-v0.1.0");
  });

  test("stable offers nothing when that release is installed", () => {
    expect(pickUpdate(releases, "stable", build("0.1.0", "stable"))).toBeUndefined();
  });

  test("beta offers the newest tagged release", () => {
    expect(tagOf(pickUpdate(releases, "beta", build("0.1.0", "stable")))).toBe("shelf-v0.2.0-beta.1");
  });

  test("an older release is not offered", () => {
    expect(pickUpdate(releases, "beta", build("0.2.0", "stable"))).toBeUndefined();
  });

  test("nightly offers the nightly built from another commit", () => {
    expect(tagOf(pickUpdate(releases, "nightly", build("0.1.0", "nightly")))).toBe("shelf-nightly");
    expect(pickUpdate(releases, "nightly", build("0.1.0", "nightly", NIGHTLY_COMMIT))).toBeUndefined();
  });

  test("off offers nothing", () => {
    expect(pickUpdate(releases, "off", build("0.0.1", "stable"))).toBeUndefined();
  });
});

describe("checkDue", () => {
  const now = 1_800_000_000_000;
  test("waits a day between checks at start", () => {
    expect(checkDue("beta", { checkedAt: now - CHECK_INTERVAL_MS + 1 }, now)).toBe(false);
    expect(checkDue("beta", { checkedAt: now - CHECK_INTERVAL_MS }, now)).toBe(true);
  });

  test("a clock set back checks again", () => {
    expect(checkDue("beta", { checkedAt: now + 1000 }, now)).toBe(true);
  });

  test("never with updates off", () => {
    expect(checkDue("off", { checkedAt: 0 }, now)).toBe(false);
  });
});
