import { mkdirSync, readdirSync } from "@pocketjs/framework/fs";
import { getOps } from "@pocketjs/framework/host";

/** Vita host extras for artwork (hosts/vita/src/art.rs and backdrop.rs). */
interface ArtHost {
  __appArt?(name: string): number;
  __artDir?: string;
  __appBackdrop?(titleId: string, file: string): number;
  __appBackdropFree?(handle: number): void;
  __artAccent?(handle: number): number;
}

const host = () => getOps() as unknown as ArtHost;

/** Where the user puts icon PNG files, as shown in the picker. */
export function artFolder(): string {
  return host().__artDir ?? "the launcher's art folder";
}

/** The launcher's data folder, which holds the art and backdrops folders. */
export function dataFolder(): string {
  return host().__artDir?.replace(/\/art$/, "") ?? "the launcher's data folder";
}

/** Where the user puts backdrop PNG files, as shown in the picker. */
export function backdropFolder(): string {
  return host().__artDir ? `${dataFolder()}/backdrops` : "the launcher's backdrops folder";
}

/** PNG files in the art folder, sorted by name. The folder is created on first use. */
export function listArt(): string[] {
  return listPng("art");
}

function listPng(folder: string): string[] {
  try {
    mkdirSync(folder);
    return readdirSync(folder)
      .filter((name) => /\.png$/i.test(name) && !name.startsWith("."))
      .sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()));
  } catch {
    return [];
  }
}

/**
 * The accent of an icon texture as the host packs it (see accent.ts), -1 when
 * the host has none for the handle.
 */
export function artAccent(handle: number): number {
  return host().__artAccent?.(handle) ?? -1;
}

/** Texture handle for an art file, -1 when the host cannot decode it. */
export function artTexture(name: string): number {
  return host().__appArt?.(name) ?? -1;
}

// Backdrop textures are large, so each one is freed once nothing draws it.
// The host hands out one handle per picture; this module counts who is using
// each handle. A handle nobody uses is kept for a while, so going back to a
// title does not decode its picture again; only the IDLE_MAX most recent are
// kept. One that falls off that list is freed a few frames later, by which
// time the node that drew it has changed or gone.
const users = new Map<number, number>();
/** Handles nobody uses, oldest first. */
const idle: number[] = [];
const freeing = new Map<number, number>();
const IDLE_MAX = 3;
const FREE_AFTER_FRAMES = 4;

/**
 * Take a title's full-screen picture: its texture handle, or -1 when it has
 * none. `file` names a PNG in the backdrops folder to use instead of the
 * title's own picture. Every handle taken has to be given back with
 * `releaseBackdrop`.
 */
export function acquireBackdrop(titleId: string, file = ""): number {
  const handle = host().__appBackdrop?.(titleId, file) ?? -1;
  if (handle < 0) return -1;
  users.set(handle, (users.get(handle) ?? 0) + 1);
  const kept = idle.indexOf(handle);
  if (kept >= 0) idle.splice(kept, 1);
  freeing.delete(handle);
  return handle;
}

/** Give back a handle from `acquireBackdrop`. -1 is ignored. */
export function releaseBackdrop(handle: number | undefined): void {
  if (handle === undefined || handle < 0) return;
  const count = (users.get(handle) ?? 0) - 1;
  if (count > 0) {
    users.set(handle, count);
    return;
  }
  users.delete(handle);
  idle.push(handle);
  while (idle.length > IDLE_MAX) {
    const oldest = idle.shift();
    if (oldest !== undefined) freeing.set(oldest, FREE_AFTER_FRAMES);
  }
}

/** Call once per frame: frees the pictures that fell off the idle list a few frames ago. */
export function pumpBackdrops(): void {
  for (const [handle, frames] of freeing) {
    if (frames > 1) {
      freeing.set(handle, frames - 1);
      continue;
    }
    freeing.delete(handle);
    host().__appBackdropFree?.(handle);
  }
}

/** PNG files in the backdrops folder, sorted by name. The folder is created on first use. */
export function listBackdrops(): string[] {
  return listPng("backdrops");
}
