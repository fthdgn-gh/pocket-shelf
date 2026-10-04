import { createSignal, Show } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import { onFrame } from "@pocketjs/framework/lifecycle";
import { scanProgress, startScan, type ScanProgress } from "../catalog.ts";
import { DEFAULT_LANGUAGE, fill, MESSAGES } from "../i18n.ts";
import { loadSettings } from "../settings.ts";
import { DEFAULT_FONT, textClasses } from "../text.ts";
import { DEFAULT_THEME, themeById } from "../themes.ts";
import { ProgressBar } from "./progress.tsx";

/** Frames the full bar stays on screen before the shelf replaces the page. */
const DONE_FRAMES = 8;
const BAR_W = 240;

/**
 * The first start's screen: the host scans the installed titles on a worker
 * thread (there is no title list yet) and this page shows how far it got.
 * It reads the saved language, theme and font itself, before the launcher's
 * state exists. `onDone` runs once the list is in memory.
 */
export function ScanPage(props: { onDone: () => void }) {
  const settings = loadSettings() ?? {};
  const t = MESSAGES[settings.language ?? DEFAULT_LANGUAGE];
  const theme = themeById(settings.theme ?? DEFAULT_THEME);
  const text = textClasses(settings.font ?? DEFAULT_FONT);
  const [progress, setProgress] = createSignal<ScanProgress>({ state: "idle" });
  const started = startScan();
  let finished = 0;
  onFrame(() => {
    const now = scanProgress();
    setProgress(now);
    // Without a scan on this host the shelf reads its table directly.
    if ((!started || now.state === "done") && ++finished === DONE_FRAMES) props.onDone();
  });
  const running = () => {
    const now = progress();
    return now.state === "running" ? now : undefined;
  };
  return (
    <View
      class="relative flex-col w-full h-full items-center justify-center gap-[10] bg-gradient-to-b from-black to-black"
      style={{ gradFrom: theme.bgTop, gradTo: theme.bgBottom }}
    >
      <Text class={text.heading} style={{ textColor: theme.text }}>
        Pocket Shelf
      </Text>
      <Text class={text.body} style={{ textColor: theme.dim }}>
        {t.scanTitle}
      </Text>
      <ProgressBar
        theme={theme}
        width={BAR_W}
        done={running()?.done ?? (progress().state === "done" ? 1 : 0)}
        total={running()?.total ?? 1}
      />
      <Show when={running()} fallback={<Text class={text.small}> </Text>}>
        {(now) => (
          <Text class={text.small} style={{ textColor: theme.faint }}>
            {fill(t.scanStep, { phase: t.scanPhases[now().phase], done: now().done, total: now().total })}
          </Text>
        )}
      </Show>
    </View>
  );
}
