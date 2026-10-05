import { readFileSync, writeFileSync } from "@pocketjs/framework/fs";
import buildInfo from "./build-info.json";

/**
 * Which releases the app offers: tagged stable releases, betas as well, the
 * `shelf-nightly` build of main, or none.
 */
export type UpdateChannel = "stable" | "beta" | "nightly" | "off";
export const UPDATE_CHANNELS: readonly UpdateChannel[] = ["stable", "beta", "nightly", "off"];

/** This build, from src/build-info.json (written by src/tools/build-info.ts). */
export interface BuildInfo {
  version: string;
  channel: "stable" | "beta" | "nightly" | "local";
  commit: string;
}
export const BUILD: BuildInfo = buildInfo as BuildInfo;

/** The channel a fresh install checks: the one it was released on; a local build checks none. */
export const DEFAULT_CHANNEL: UpdateChannel = BUILD.channel === "local" ? "off" : BUILD.channel;

const REPOSITORY = "fthdgn-gh/pocket-shelf";
/** The newest releases, newest first. Ten reach past any run of betas. */
export const RELEASES_URL = `https://api.github.com/repos/${REPOSITORY}/releases?per_page=10`;
export const NIGHTLY_TAG = "shelf-nightly";
const VPK_NAME = "pocket-shelf.vpk";
/** Where the download goes, under the data folder. hosts/vita/src/http.rs allows only this name. */
export const UPDATE_FILE = "update/pocket-shelf.vpk";

/** A release that can be installed. */
export interface Release {
  tag: string;
  /** "0.1.0-beta.1"; a nightly has none. */
  version?: string;
  /** The commit a nightly was built from; tagged releases do not carry it. */
  commit?: string;
  title: string;
  /** "2026-10-05", the day it was published. */
  date: string;
  url: string;
  size: number;
}

interface GitHubRelease {
  tag_name?: unknown;
  name?: unknown;
  draft?: unknown;
  prerelease?: unknown;
  target_commitish?: unknown;
  published_at?: unknown;
  assets?: { name?: unknown; browser_download_url?: unknown; size?: unknown }[];
}

const TAG = /^shelf-v(\d+\.\d+\.\d+(?:-beta\.\d+)?)$/;

/**
 * The installable releases of a reply to RELEASES_URL: published, with the
 * VPK attached, and tagged `shelf-v<version>` or `shelf-nightly`. Null for a
 * reply that is not the list GitHub sends.
 */
export function parseReleases(text: string): Release[] | null {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return null;
  }
  if (!Array.isArray(raw)) return null;
  const releases: Release[] = [];
  for (const item of raw as GitHubRelease[]) {
    if (!item || typeof item !== "object" || item.draft === true || typeof item.tag_name !== "string") continue;
    const asset = item.assets?.find((entry) => entry.name === VPK_NAME);
    const url = asset?.browser_download_url;
    if (typeof url !== "string" || !url.startsWith("https://") || typeof asset?.size !== "number") continue;
    const tag = item.tag_name;
    const version = TAG.exec(tag)?.[1];
    const commit = typeof item.target_commitish === "string" ? item.target_commitish : "";
    if (!version && !(tag === NIGHTLY_TAG && /^[0-9a-f]{40}$/.test(commit))) continue;
    releases.push({
      tag,
      version,
      commit: version ? undefined : commit,
      title: typeof item.name === "string" && item.name ? item.name : tag,
      date: typeof item.published_at === "string" ? item.published_at.slice(0, 10) : "",
      url,
      size: asset.size,
    });
  }
  return releases;
}

/** -1, 0 or 1 as `a` is older than, the same as or newer than `b`. A beta comes before its release. */
export function compareVersions(a: string, b: string): number {
  const parse = (version: string) => {
    const [core = "", beta] = version.split("-beta.");
    const parts = core.split(".").map((part) => Number(part) || 0);
    return [...parts, beta === undefined ? Number.MAX_SAFE_INTEGER : Number(beta) || 0];
  };
  const left = parse(a);
  const right = parse(b);
  for (let index = 0; index < Math.max(left.length, right.length); index++) {
    const difference = (left[index] ?? 0) - (right[index] ?? 0);
    if (difference !== 0) return Math.sign(difference);
  }
  return 0;
}

/**
 * The release the channel offers over `build`, or undefined when there is
 * none or it is what is installed. Stable takes the newest release without a
 * beta number, beta the newest of all tagged releases; either only when it is
 * newer than the installed version. Nightly takes `shelf-nightly` when it was
 * built from another commit.
 */
export function pickUpdate(releases: Release[], channel: UpdateChannel, build: BuildInfo): Release | undefined {
  if (channel === "off") return undefined;
  if (channel === "nightly") {
    const nightly = releases.find((release) => release.tag === NIGHTLY_TAG);
    return nightly && nightly.commit !== build.commit ? nightly : undefined;
  }
  const newest = releases
    .filter((release) => release.version && (channel === "beta" || !release.version.includes("-beta.")))
    .sort((a, b) => compareVersions(b.version!, a.version!))[0];
  return newest && compareVersions(newest.version!, build.version) > 0 ? newest : undefined;
}

/** A name for a release in the drawer: "0.1.0-beta.1", or the nightly's day and commit. */
export function releaseLabel(release: Release): string {
  return release.version ?? `${release.date} (${release.commit?.slice(0, 7) ?? ""})`;
}

/** The installed build, in the same form. */
export function buildLabel(build: BuildInfo): string {
  return build.channel === "nightly" || build.channel === "local"
    ? `${build.version} (${build.commit.slice(0, 7) || build.channel})`
    : build.version;
}

// --- When to check -----------------------------------------------------------

/** A check at start waits this long after the last one. */
export const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

/** What update.json in the data folder keeps between starts. */
export interface UpdateRecord {
  /** When the last check finished, in ms since 1970. */
  checkedAt: number;
  /** A release the user put off: a check at start does not offer it again. */
  later?: string;
}

const RECORD_FILE = "update.json";

export function loadUpdateRecord(): UpdateRecord {
  try {
    const raw = JSON.parse(readFileSync(RECORD_FILE, "utf8")) as Partial<UpdateRecord>;
    return {
      checkedAt: typeof raw.checkedAt === "number" ? raw.checkedAt : 0,
      later: typeof raw.later === "string" ? raw.later : undefined,
    };
  } catch {
    return { checkedAt: 0 };
  }
}

export function saveUpdateRecord(record: UpdateRecord): void {
  try {
    writeFileSync(RECORD_FILE, JSON.stringify(record));
  } catch (error) {
    console.log(`Update record not saved: ${error}`);
  }
}

/** Whether a check at start is due: a channel is chosen and the last check is a day old. */
export function checkDue(channel: UpdateChannel, record: UpdateRecord, now: number): boolean {
  return channel !== "off" && (now - record.checkedAt >= CHECK_INTERVAL_MS || now < record.checkedAt);
}

/** The key `later` stores: the tag, and for a nightly its commit, since its tag stays the same. */
export const releaseKey = (release: Release): string => `${release.tag}@${release.commit ?? ""}`;
