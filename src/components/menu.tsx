import { For } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import { FONTS } from "../fonts.ts";
import { DETAIL_LABELS, VIEW_LABELS } from "../navigation.ts";
import type { LauncherState } from "../state.ts";

/** The SELECT menu: a centered panel over a dimmed screen. */
export function MenuOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const rows = () => [
    { label: "Theme", value: state.theme().name },
    { label: "Font", value: FONTS.find((item) => item.id === state.font())?.name ?? "" },
    { label: "View", value: VIEW_LABELS[state.view()] },
    { label: "Details", value: DETAIL_LABELS[state.detail()] },
    { label: "Categories", value: "Manage ►" },
    { label: "Confirm button", value: state.confirmGlyph() },
  ];
  const hint = () => `D-pad: Move / Change  ·  ${state.cancelGlyph()}: Close`;
  return (
    <View class="absolute inset-0 items-center justify-center bg-[#000000b3]">
      <View class={state.theme().menuPanel}>
        <Text class={state.theme().menuTitle}>Menu</Text>
        <For each={rows()}>
          {(row, index) => (
            <View class={state.menuRow() === index() ? state.theme().menuRowActive : state.theme().menuRow}>
              <Text class={state.theme().menuLabel}>{row.label}</Text>
              <Text class={state.theme().menuValue}>{`◄ ${row.value} ►`}</Text>
            </View>
          )}
        </For>
        <Text class={state.theme().menuHint}>{hint()}</Text>
      </View>
    </View>
  );
}
