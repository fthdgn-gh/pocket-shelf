import { Match, Show, Switch } from "solid-js";
import { View } from "@pocketjs/framework/components";
import { dataFolder } from "../art-files.ts";
import { fitTitle } from "../catalog.ts";
import { fill } from "../i18n.ts";
import type { LauncherState } from "../state.ts";
import { MODE_ROW, SOURCE_ROW, START_ROW } from "../scrape.ts";
import { SCRAPER_NAMES, type ScraperId } from "../scrapers.ts";
import { fontSlot } from "../text.ts";
import { hints, Prompt } from "./icons.tsx";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

/** Fetch artwork for many titles: choose which, then watch the run. */
export function ScrapeOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const scrape = state.scrape;
  const t = state.t;
  const fit = (text: string, width: number) => fitTitle(text, width, fontSlot(state.font(), "bodyBold"));
  // What the setup's choices do: per source, missing pictures or all of them.
  const help = () => {
    const all = scrape.replace();
    if (scrape.source() === "libretro") return all ? t().scrapeHelpLibretroAll : t().scrapeHelpLibretro;
    if (scrape.source() === "auto") return all ? t().scrapeHelpAutoAll : t().scrapeHelpAuto;
    return all ? t().scrapeHelpAll : t().scrapeHelp;
  };
  // What the run has found so far, under the title it is on.
  const Counts = () => (
    <>
      <OptionRow state={state} active={false} label={t().icons} value={String(scrape.found().icon)} />
      <OptionRow state={state} active={false} label={t().backdrops} value={String(scrape.found().backdrop)} />
      <OptionRow state={state} active={false} label={t().notFound} value={String(scrape.missing())} />
    </>
  );
  return (
    <Drawer state={state} title={t().fetchArt} size="wide">
      <Switch>
        <Match when={scrape.step() === "setup"}>
          <OptionRow
            state={state}
            active={scrape.row() === 0}
            label={t().scrapeScope}
            value={fit(scrape.scope()?.label ?? "", 150)}
            step="both"
          />
          <OptionRow
            state={state}
            active={scrape.row() === SOURCE_ROW}
            label={t().scrapeSource}
            value={scrape.source() === "auto" ? t().sourceAuto : SCRAPER_NAMES[scrape.source() as ScraperId]}
            step="both"
          />
          <OptionRow
            state={state}
            active={scrape.row() === MODE_ROW}
            label={t().scrapeMode}
            value={scrape.replace() ? t().everything : t().onlyMissing}
            step="both"
          />
          <OptionRow
            state={state}
            active={scrape.row() === START_ROW}
            label={t().start}
            value={fill(scrape.replace() ? t().scrapeCountAll : t().scrapeCount, { count: scrape.wanted().length })}
            step="right"
          />
          <Spacer />
          <Note state={state}>{scrape.status() || help()}</Note>
          <Prompt
            state={state}
            parts={hints(
              [state.confirmButton(), scrape.row() === START_ROW ? t().start : t().change],
              [state.cancelButton(), t().back],
            )}
          />
        </Match>

        <Match when={scrape.step() === "key"}>
          <OptionRow state={state} active label={t().enterKey} step="right" />
          <View class="h-[4] shrink-0" />
          <Note state={state}>{t().keyHelp}</Note>
          <Note state={state}>{`${dataFolder()}/steamgriddb.txt`}</Note>
          <Spacer />
          <Show when={scrape.status()}>
            <Note state={state}>{scrape.status()}</Note>
          </Show>
          <Prompt state={state} parts={hints([state.confirmButton(), t().type], [state.cancelButton(), t().back])} />
        </Match>

        <Match when={scrape.step() === "running" || scrape.step() === "done"}>
          {/* The title being fetched for; once the run ends, the titles it covered. */}
          <Show
            when={scrape.current()}
            fallback={
              <OptionRow state={state} active label={t().scrapeScope} value={fit(scrape.scope()?.label ?? "", 170)} />
            }
          >
            <OptionRow state={state} active label={t().title} value={fit(scrape.current(), 190)} />
          </Show>
          <OptionRow
            state={state}
            active={false}
            label={t().progress}
            value={`${scrape.done()} / ${scrape.total()}`}
          />
          <Counts />
          <Spacer />
          <Show when={scrape.status()}>
            <Note state={state}>{scrape.status()}</Note>
          </Show>
          <Prompt
            state={state}
            parts={hints([state.cancelButton(), scrape.step() === "running" ? t().stop : t().close])}
          />
        </Match>
      </Switch>
    </Drawer>
  );
}
