// PSP and PS1 titles with no bubble start through Adrenaline, with Pocket
// Shelf's boot plugin (src/psp-boot/) loaded at Adrenaline's XMB. The Vita
// host reads Adrenaline's plugin lists and changes them (hosts/vita/src/pspemu.rs);
// the app asks before a launch and changes them only once the user agrees.
import { getOps } from "@pocketjs/framework/host";

/**
 * Whether Adrenaline loads the plugin: not installed, not in its plugin
 * lists, turned off there, or on.
 */
export type PluginState = "noAdrenaline" | "missing" | "off" | "on";

interface AdrenalineHost {
  __adrPlugin?(): string;
  __adrPluginEnable?(): number;
}

const host = (): AdrenalineHost => getOps() as unknown as AdrenalineHost;

/** The plugin's state. A host without the extra counts as on, so it launches as before. */
export function pluginState(): PluginState {
  const state = host().__adrPlugin?.();
  return state === "noAdrenaline" || state === "missing" || state === "off" ? state : "on";
}

/** Add the plugin to Adrenaline's lists, or turn it back on. Returns whether it is now on. */
export function enablePlugin(): boolean {
  return (host().__adrPluginEnable?.() ?? 0) === 1;
}
