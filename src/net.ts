import { getOps } from "@pocketjs/framework/host";

// HTTPS requests through the Vita host's extras (hosts/vita/src/http.rs). The
// host runs each request on its own thread; `pumpNet` asks it once per frame
// how the open ones are doing and reports the finished ones.

interface NetHost {
  __netGet?(url: string, authorization: string): number;
  __netSave?(url: string, path: string): number;
  __netState?(id: number): string;
  __netText?(id: number): string;
  __netClose?(id: number): void;
}

const host = () => getOps() as unknown as NetHost;

export type NetResult =
  | { ok: true; status: number; text: string }
  /** `error` is the host's reason, for example "dns", "connect" or "tls ...". */
  | { ok: false; error: string };

interface Pending {
  id: number;
  done: (result: NetResult) => void;
  progress?: (received: number, total: number) => void;
}

const pending: Pending[] = [];

/** Whether this host can make requests at all. */
export function netAvailable(): boolean {
  return typeof host().__netGet === "function";
}

function track(id: number | undefined, done: Pending["done"], progress?: Pending["progress"]): () => void {
  if (id === undefined || id < 0) {
    done({ ok: false, error: netAvailable() ? "the request could not be started" : "no network on this device" });
    return () => {};
  }
  const entry: Pending = { id, done, progress };
  pending.push(entry);
  return () => {
    const index = pending.indexOf(entry);
    if (index < 0) return;
    pending.splice(index, 1);
    host().__netClose?.(id);
  };
}

/**
 * GET `url` and keep the reply as text. `authorization` is the header's value,
 * "" for none. Returns a function that cancels the request.
 */
export function getText(url: string, authorization: string, done: (result: NetResult) => void): () => void {
  return track(host().__netGet?.(url, authorization), done);
}

/**
 * Download a PNG to `path` under the data folder (`art/...` or
 * `backdrops/...`). A status other than 200 writes no file.
 */
export function saveFile(
  url: string,
  path: string,
  done: (result: NetResult) => void,
  progress?: (received: number, total: number) => void,
): () => void {
  return track(host().__netSave?.(url, path), done, progress);
}

/** Call once per frame: reports progress and finishes completed requests. */
export function pumpNet(): void {
  for (const entry of [...pending]) {
    const state = host().__netState?.(entry.id) ?? "error no network";
    const [kind, ...rest] = state.split(" ");
    if (kind === "busy") {
      entry.progress?.(Number(rest[0]) || 0, Number(rest[1]) || 0);
      continue;
    }
    pending.splice(pending.indexOf(entry), 1);
    const result: NetResult =
      kind === "done"
        ? { ok: true, status: Number(rest[0]) || 0, text: host().__netText?.(entry.id) ?? "" }
        : { ok: false, error: rest.join(" ") || "unknown" };
    host().__netClose?.(entry.id);
    entry.done(result);
  }
}
