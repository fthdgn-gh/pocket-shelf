import { createSignal, Show } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import { onFrame } from "@pocketjs/framework/lifecycle";
import { scanProgress, startScan, type ScanProgress } from "../catalog.ts";
import { DEFAULT_LANGUAGE, fill, MESSAGES } from "../i18n.ts";
import { loadSettings } from "../settings.ts";
import { textClasses } from "../text.ts";
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
  const text = textClasses();
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
      <Mark text={theme.text} />
      <View class="h-[2] shrink-0" />
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

/**
 * The app's mark: the Home view in small, a selected tile in its ring and two
 * smaller tiles after it. `src/tools/art.ts` draws the same mark (`MARK`) on
 * the splash and the LiveArea pictures; keep the two in step.
 */
function Mark(props: { text: string }) {
  return (
    <View class="relative w-[110] h-[48] shrink-0">
      <View class="absolute left-[52] top-[4] w-[26] h-[26] rounded-[2] opacity-80 bg-gradient-to-b from-amber-400 to-red-600" />
      <View class="absolute left-[84] top-[4] w-[26] h-[26] rounded-[2] opacity-80 bg-gradient-to-b from-emerald-400 to-teal-700" />
      <View class="absolute left-[4] top-[4] w-[40] h-[40] rounded-[3] bg-gradient-to-b from-violet-400 to-violet-800" />
      <View class="absolute left-0 top-0 w-[48] h-[48] rounded-[3] border-2" style={{ borderColor: props.text }} />
    </View>
  );
}
