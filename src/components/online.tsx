import { For, Match, Show, Switch } from "solid-js";
import { Image, View } from "@pocketjs/framework/components";
import { dataFolder } from "../art-files.ts";
import { fitTitle } from "../catalog.ts";
import { ASSET_KINDS, ONLINE_ROWS } from "../online.ts";
import type { LauncherState } from "../state.ts";
import type { AssetKind } from "../steamgriddb.ts";
import { fontSlot } from "../text.ts";
import { hints, Prompt } from "./icons.tsx";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

// A candidate at the size it is judged at: an icon as a tile, a backdrop in
// the screen's shape.
const PREVIEW: Record<AssetKind, string> = {
  icon: "w-[96] h-[96] shrink-0",
  backdrop: "w-[212] h-[120] shrink-0",
};

/** SteamGridDB: search a game, then step through its icons and backdrops. */
export function OnlineOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const online = state.online;
  const t = state.t;
  const kindLabel = (which: AssetKind) => (which === "icon" ? t().icon : t().backdrop);
  const fit = (text: string, width: number) => fitTitle(text, width, fontSlot(state.font(), "bodyBold"));

  // Games list: the search row, then the games; it scrolls with the highlight.
  const entries = () => [
    { label: t().search, value: fit(online.term(), 170) },
    ...online.games().map((game) => ({
      label: fit(game.name, game.year ? 200 : 240),
      value: game.year ? String(game.year) : "",
    })),
  ];
  const first = () => {
    const last = entries().length - ONLINE_ROWS;
    return Math.max(0, Math.min(online.row() - Math.floor(ONLINE_ROWS / 2), last));
  };
  const visible = () => entries().slice(first(), first() + ONLINE_ROWS);

  const kind = (): AssetKind => ASSET_KINDS[online.row()] ?? "icon";
  const position = (which: AssetKind) => {
    const count = online.assets()[which].length;
    return count === 0 ? t().none : `${online.index()[which] + 1} / ${count}`;
  };

  return (
    <Drawer state={state} title="SteamGridDB" size="wide">
      <Switch>
        <Match when={online.step() === "key"}>
          <OptionRow state={state} active label={t().enterKey} step="right" />
          <View class="h-[4] shrink-0" />
          <Note state={state}>{t().keyHelp}</Note>
          <Note state={state}>{`${dataFolder()}/steamgriddb.txt`}</Note>
          <Spacer />
          <Show when={online.status()}>
            <Note state={state}>{online.status()}</Note>
          </Show>
          <Prompt state={state} parts={hints([state.confirmButton(), t().type], [state.cancelButton(), t().back])} />
        </Match>

        <Match when={online.step() === "games"}>
          <For each={visible()}>
            {(entry, index) => (
              <OptionRow
                state={state}
                active={first() + index() === online.row()}
                label={entry.label}
                value={entry.value}
                step={first() + index() === 0 ? "right" : undefined}
              />
            )}
          </For>
          <Spacer />
          <Note state={state}>{online.status()}</Note>
          <Prompt
            state={state}
            parts={hints(
              [state.confirmButton(), online.row() === 0 ? t().edit : t().choose],
              [state.cancelButton(), t().back],
            )}
          />
        </Match>

        <Match when={online.step() === "assets"}>
          <For each={ASSET_KINDS}>
            {(which, index) => (
              <OptionRow
                state={state}
                active={online.row() === index()}
                label={kindLabel(which)}
                value={position(which)}
                step="both"
              />
            )}
          </For>
          <View class="flex-col items-center justify-center w-full h-[128] shrink-0">
            <Show when={online.preview()[kind()]} keyed>
              {(src) => <Image class={PREVIEW[kind()]} src={src} />}
            </Show>
          </View>
          <Spacer />
          <Note state={state}>{online.status() || fit(online.chosen()?.name ?? "", 250)}</Note>
          <Prompt
            state={state}
            parts={hints(
              [state.confirmButton(), t().use],
              ["dpadHorizontal", t().next],
              [state.cancelButton(), t().back],
            )}
          />
        </Match>
      </Switch>
    </Drawer>
  );
}
