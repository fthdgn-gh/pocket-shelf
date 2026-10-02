import { For, Show } from "solid-js";
import { Image, View } from "@pocketjs/framework/components";
import { artFolder, backdropFolder } from "../art-files.ts";
import { fitTitle } from "../catalog.ts";
import { PICKER_ROWS, type LauncherState } from "../state.ts";
import { fontSlot } from "../text.ts";
import { hints, Prompt } from "./icons.tsx";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

// The preview: an icon as a square, a backdrop in the screen's shape, each
// with a 2 px frame.
const FRAME = {
  icon: "absolute left-[32] top-[70] w-[132] h-[132] items-center justify-center",
  backdrop: "absolute left-[8] top-[84] w-[180] h-[104] items-center justify-center",
};
const PICTURE = {
  icon: "w-[128] h-[128] shrink-0",
  backdrop: "w-[176] h-[100] shrink-0",
};

/**
 * Lists PNG files of the art folder after "Default" and "Game icon", or of
 * the backdrops folder after "Default" and "None": the ones named after the
 * title, or all of them (triangle). The highlighted row's picture shows
 * beside the drawer.
 */
export function ArtPickerOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const backdrop = () => state.pickerKind() === "backdrop";
  const t = state.t;
  const entries = () => [t().standard, backdrop() ? t().none : t().gameIcon, ...state.artFiles()];
  // Scroll so the highlighted row stays in view.
  const first = () => {
    const last = entries().length - PICKER_ROWS;
    return Math.max(0, Math.min(state.artRow() - Math.floor(PICKER_ROWS / 2), last));
  };
  const visible = () => entries().slice(first(), first() + PICKER_ROWS);
  return (
    <Drawer
      state={state}
      title={backdrop() ? t().backdrop : t().boxArt}
      size="wide"
      aside={
        // The highlighted row's picture, centered in the 196 px left of the drawer.
        <Show when={state.pickerPreview()} keyed>
          {(src) => (
            <View class={backdrop() ? FRAME.backdrop : FRAME.icon} style={{ bgColor: state.theme().tile }}>
              <Image class={backdrop() ? PICTURE.backdrop : PICTURE.icon} src={src} />
            </View>
          )}
        </Show>
      }
    >
      <For each={visible()}>
        {(name, index) => (
          <OptionRow
            state={state}
            active={first() + index() === state.artRow()}
            label={fitTitle(name, 240, fontSlot(state.font(), "bodyBold"))}
          />
        )}
      </For>
      <Spacer />
      <Show when={state.artNote()}>
        <Note state={state}>{state.artNote()}</Note>
      </Show>
      <Show when={!state.artNote() && state.artFiles().length === 0}>
        <Note state={state}>{t().noPng}</Note>
        <Note state={state}>{backdrop() ? backdropFolder() : artFolder()}</Note>
      </Show>
      <Prompt
        state={state}
        parts={hints(
          [state.confirmButton(), t().use],
          ["triangle", state.pickerEvery() ? t().thisTitle : t().allFiles],
          [state.cancelButton(), t().back],
        )}
      />
    </Drawer>
  );
}
