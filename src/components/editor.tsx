import { For, Show } from "solid-js";
import { fitTitle } from "../catalog.ts";
import { fill } from "../i18n.ts";
import { NO_BACKDROP } from "../overrides.ts";
import type { LauncherState } from "../state.ts";
import { fontSlot } from "../text.ts";
import { hints, Prompt } from "./icons.tsx";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

// Room for a value beside its label in a wide drawer, with its arrow.
const VALUE_W = 160;

/** Per-title editor: favorite, category, title, box art, backdrop, SteamGridDB and reset. */
export function EditorOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const t = state.t;
  const fit = (value: string) => fitTitle(value, VALUE_W, fontSlot(state.font(), "bodyBold"));
  const art = () => {
    const game = state.editorGame();
    if (game?.artIcon) return t().gameIcon;
    if (!game?.art) return t().standard;
    return game.artAuto ? fill(t().autoArt, { file: game.art }) : game.art;
  };
  const backdrop = () => {
    const file = state.editorGame()?.backdrop;
    if (file === NO_BACKDROP) return t().none;
    return file ?? t().standard;
  };
  const rows = () => {
    const game = state.editorGame();
    const category = state.categoryLabel(game?.category ?? "");
    return [
      { label: t().favorite, value: game?.favorite ? t().on : t().off, step: "both" as const },
      { label: t().category, value: category, step: "both" as const },
      { label: t().title, value: fit(game?.title ?? ""), step: "right" as const },
      { label: t().boxArt, value: fit(art()), step: "right" as const },
      { label: t().backdrop, value: fit(backdrop()), step: "right" as const },
      { label: "SteamGridDB", value: t().search, step: "right" as const },
      { label: t().reset, value: "", step: undefined },
    ];
  };
  return (
    <Drawer state={state} title={t().editorTitle} size="wide">
      <For each={rows()}>
        {(row, index) => (
          <OptionRow
            state={state}
            active={state.editorRow() === index()}
            label={row.label}
            value={row.value}
            step={row.step}
          />
        )}
      </For>
      <Spacer />
      <Show
        when={state.editorNote()}
        fallback={<Prompt state={state} parts={hints([state.confirmButton(), t().change], [state.cancelButton(), t().back])} />}
      >
        <Note state={state}>{state.editorNote()}</Note>
      </Show>
    </Drawer>
  );
}
