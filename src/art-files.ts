import { mkdirSync, readdirSync } from "@pocketjs/framework/fs";
import { getOps } from "@pocketjs/framework/host";

/** Vita host extras for custom box art (hosts/vita/src/art.rs). */
interface ArtHost {
  __appArt?(name: string): number;
  __artDir?: string;
}

const host = () => getOps() as unknown as ArtHost;

/** Where the user puts PNG files, as shown in the picker. */
export function artFolder(): string {
  return host().__artDir ?? "the launcher's art folder";
}

/** PNG files in the art folder, sorted by name. The folder is created on first use. */
export function listArt(): string[] {
  try {
    mkdirSync("art");
    return readdirSync("art")
      .filter((name) => /\.png$/i.test(name) && !name.startsWith("."))
      .sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()));
  } catch {
    return [];
  }
}

/** Texture handle for an art file, -1 when the host cannot decode it. */
export function artTexture(name: string): number {
  return host().__appArt?.(name) ?? -1;
}
