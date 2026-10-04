import { Show } from "solid-js";
import { fill } from "../i18n.ts";
import type { LauncherState } from "../state.ts";
import { hints, Prompt } from "./icons.tsx";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

/**
 * Shown when a title without a bubble is started and Adrenaline would not load
 * Pocket Shelf's boot plugin: offers to add the plugin to Adrenaline's lists,
 * or to turn it back on. Without Adrenaline it only says so.
 */
export function AdrenalineOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const t = state.t;
  const title = () => state.games()[state.selectedIndex()]?.title ?? "";
  const plugin = () => state.adrPlugin();
  const canFix = () => plugin() === "missing" || plugin() === "off";
  const note = () => {
    if (state.adrFailed()) return t().adrFailed;
    const text = plugin() === "off" ? t().adrOff : plugin() === "missing" ? t().adrMissing : t().adrNone;
    return fill(text, { title: title() });
  };
  return (
    <Drawer state={state} title={t().adrTitle} size="wide">
      <Show when={canFix()}>
        <OptionRow state={state} active label={plugin() === "off" ? t().adrTurnOn : t().adrAdd} step="right" />
      </Show>
      <Spacer />
      <Note state={state}>{note()}</Note>
      <Prompt
        state={state}
        parts={
          canFix()
            ? hints([state.confirmButton(), plugin() === "off" ? t().adrTurnOn : t().adrAdd], [state.cancelButton(), t().back])
            : hints([state.cancelButton(), t().back])
        }
      />
    </Drawer>
  );
}
