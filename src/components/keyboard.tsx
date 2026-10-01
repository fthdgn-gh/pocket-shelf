import { For } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import { fitTail } from "../catalog.ts";
import { fontSlot } from "../fonts.ts";
import type { LauncherState } from "../state.ts";

// Space inside the text field: the panel (430) less its padding and the row's.
const FIELD_TEXT_W = 380;

/** On-screen keyboard for a custom title. */
export function KeyboardOverlay(props: { state: LauncherState }) {
  const { state } = props;
  // A title longer than the field shows its end, where the caret is.
  const shown = () =>
    fitTail(`${state.keyboardText()}_`, FIELD_TEXT_W, fontSlot(state.font(), 14, true));
  const hintKeys = () => `${state.confirmGlyph()}: Key  □: Delete  L: Shift  R: Symbols`;
  const hintExit = () => `START: Done  ${state.cancelGlyph()}: Cancel`;
  const label = (key: { action: string; lower: string; upper: string }) =>
    key.action === "symbols" ? (state.symbols() ? "Abc" : "Symbols") : state.shift() ? key.upper : key.lower;
  return (
    <View class="absolute inset-0 items-center justify-center bg-[#000000e6]">
      <View class={state.theme().panelWide}>
        <View class={state.theme().menuRow}>
          <Text class={state.theme().menuValue}>{shown()}</Text>
        </View>
        <For each={state.keyRows()}>
          {(row, rowIndex) => (
            <View class="flex-row justify-center gap-1">
              <For each={row}>
                {(key, colIndex) => {
                  const active = () => state.keyRow() === rowIndex() && state.keyCol() === colIndex();
                  const box = () =>
                    key.wide
                      ? active() ? state.theme().keyWideActive : state.theme().keyWide
                      : active() ? state.theme().keyActive : state.theme().key;
                  return (
                    <View class={box()}>
                      <Text class={active() ? state.theme().tabTextActive : state.theme().tabText}>
                        {label(key)}
                      </Text>
                    </View>
                  );
                }}
              </For>
            </View>
          )}
        </For>
        <Text class={state.theme().menuHint}>{hintKeys()}</Text>
        <Text class={state.theme().menuHint}>{hintExit()}</Text>
      </View>
    </View>
  );
}
