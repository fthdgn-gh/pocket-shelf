import { Show } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import type { LauncherState } from "../state.ts";
import { formatClock, wifiLevel } from "../status.ts";
import { alpha, isLightTheme } from "../themes.ts";
import { Icon, type IconName } from "./icons.tsx";

/** Battery body, which the charge fills from the left. The class literals below spell it. */
const BATTERY_W = 24;
/** Corner radius of the body and of the charge. */
const BATTERY_R = 3;
/** At or below this charge the battery is drawn in `LOW_BATTERY`. */
const LOW_PERCENT = 15;
const LOW_BATTERY = "#f87171";

const WIFI: Record<"night" | "day", IconName[]> = {
  night: ["wifi0", "wifi1", "wifi2", "wifi3"],
  day: ["wifi0Day", "wifi1Day", "wifi2Day", "wifi3Day"],
};

/**
 * The strip above the category header: the time on the left; Wi-Fi,
 * Bluetooth and the battery on the right. A reading the host cannot give is
 * left out. Wi-Fi shows only while connected, Bluetooth only while it is
 * switched on.
 */
export function StatusBar(props: { state: LauncherState }) {
  const { state } = props;
  const theme = state.theme;
  const status = state.status;
  const ink = () => (isLightTheme(theme()) ? "day" : "night");
  // 0 is a reading too (an empty battery), so this tests for null.
  const wifi = () => status()?.wifi ?? 0;
  const battery = () => status()?.battery ?? null;
  const clock = () => {
    const now = status();
    if (!now) return "";
    const format = state.clockFormat();
    return formatClock(now.hour, now.minute, format === "system" ? now.clock24 : format === "24");
  };
  return (
    <View class="flex-row items-center justify-between w-full h-[16] shrink-0 px-4 pt-[3]">
      <Text class={state.text().smallBold} style={{ textColor: theme().dim }}>
        {clock()}
      </Text>
      <View class="flex-row items-center gap-[6]">
        <Show when={wifi() > 0}>
          <Icon name={WIFI[ink()][wifiLevel(wifi())]!} />
        </Show>
        <Show when={status()?.bluetooth}>
          <Icon name={ink() === "day" ? "bluetoothDay" : "bluetooth"} />
        </Show>
        <Show when={battery() !== null}>
          <Battery state={state} percent={battery() ?? 0} charging={status()?.charging ?? false} />
        </Show>
      </View>
    </View>
  );
}

/**
 * A rounded body filled from the left to the charge, with the number over
 * it (the percent setting) and a nub on the right. The charge is drawn in
 * the text color, the accent while charging, red when low.
 */
function Battery(props: { state: LauncherState; percent: number; charging: boolean }) {
  const { state } = props;
  const theme = state.theme;
  const fillW = () => Math.round((BATTERY_W * props.percent) / 100);
  const fill = () =>
    props.charging ? theme().accent : props.percent <= LOW_PERCENT ? LOW_BATTERY : theme().text;
  const empty = () => alpha(theme().text, "59");
  // The number stands out from the charge: dark on the dark themes' light charge, light on Daylight's.
  const number = () => (isLightTheme(theme()) ? theme().panel : theme().bgBottom);
  return (
    <View class="flex-row items-center gap-[1]">
      <View class="relative w-[24] h-[12] rounded-[3]" style={{ bgColor: empty() }}>
        {/* A charge wider than both corners is a rounded bar with a square right end. */}
        <Show when={fillW() >= BATTERY_R * 2}>
          <View class="absolute left-0 top-0 h-[12] rounded-[3]" style={{ width: fillW(), bgColor: fill() }} />
          <Show when={fillW() <= BATTERY_W - BATTERY_R}>
            <View
              class="absolute top-0 h-[12]"
              style={{ insetL: fillW() - BATTERY_R, width: BATTERY_R, bgColor: fill() }}
            />
          </Show>
        </Show>
        <Show when={fillW() > 0 && fillW() < BATTERY_R * 2}>
          <View class="absolute left-0 top-0 h-[12] rounded-[2]" style={{ width: fillW(), bgColor: fill() }} />
        </Show>
        <Show when={state.batteryPercentOn()}>
          <View class="absolute left-0 top-0 w-[24] h-[12] flex-row items-center justify-center">
            <Text class={state.text().smallBold} style={{ textColor: number() }}>
              {String(props.percent)}
            </Text>
          </View>
        </Show>
      </View>
      <View class="w-[2] h-[4] rounded-[1]" style={{ bgColor: empty() }} />
    </View>
  );
}
