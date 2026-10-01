import { For, onMount } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { fitTail } from "../catalog.ts";
import type { LauncherState } from "../state.ts";
import { fontSlot } from "../text.ts";
import { hints, Prompt } from "./icons.tsx";

// Space inside the text field: the key rows are 416 wide, less the field's padding.
const FIELD_TEXT_W = 396;

/** On-screen keyboard: a sheet that rises from the bottom edge. */
export function KeyboardOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const theme = state.theme;
  let sheetRef: NodeMirror | undefined;
  onMount(() => {
    if (sheetRef) animate(sheetRef, "translateY", 0, { dur: 180, easing: "out" });
  });
  // Text longer than the field shows its end, where the caret is.
  const shown = () => fitTail(`${state.keyboardText()}_`, FIELD_TEXT_W, fontSlot(state.font(), "bodyBold"));
  const label = (key: { action: string; lower: string; upper: string }) =>
    key.action === "symbols" ? (state.symbols() ? "Abc" : "Symbols") : state.shift() ? key.upper : key.lower;
  return (
    <View class="absolute inset-0" style={{ bgColor: theme().scrim }}>
      <View
        ref={(el) => {
          sheetRef = el;
        }}
        class="absolute left-0 right-0 bottom-0 flex-col items-center gap-1 pt-3 pb-2"
        style={{ bgColor: theme().panel, translateY: 80 }}
      >
        <View class="absolute top-0 left-0 right-0 h-[2]" style={{ bgColor: theme().accent }} />
        <View class="flex-row items-center w-[416] h-[26] px-[10] rounded-md" style={{ bgColor: theme().tile }}>
          <Text class={state.text().bodyBold} style={{ textColor: theme().text }}>
            {shown()}
          </Text>
        </View>
        <View class="h-[2] shrink-0" />
        <For each={state.keyRows()}>
          {(row, rowIndex) => (
            <View class="flex-row justify-center gap-1">
              <For each={row}>
                {(key, colIndex) => {
                  const active = () => state.keyRow() === rowIndex() && state.keyCol() === colIndex();
                  return (
                    <View
                      class={
                        key.wide
                          ? "items-center justify-center w-[80] h-[24] rounded-md"
                          : "items-center justify-center w-[38] h-[24] rounded-md"
                      }
                      style={{ bgColor: active() ? theme().accent : theme().tile }}
                    >
                      <Text
                        class={active() ? state.text().smallBold : state.text().small}
                        style={{ textColor: active() ? theme().panel : theme().text }}
                      >
                        {label(key)}
                      </Text>
                    </View>
                  );
                }}
              </For>
            </View>
          )}
        </For>
        <View class="h-[2] shrink-0" />
        <Prompt
          state={state}
          parts={hints(
            [state.confirmButton(), "Key"],
            ["square", "Delete"],
            ["l", "Shift"],
            ["r", "Symbols"],
            ["start", "Done"],
            [state.cancelButton(), "Cancel"],
          )}
        />
      </View>
    </View>
  );
}
