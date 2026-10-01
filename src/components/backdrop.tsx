import { onMount, Show } from "solid-js";
import { Image, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import type { LauncherState } from "../state.ts";
import { alpha } from "../themes.ts";

// The picture covers the whole 480x272 screen. The host stores it as a
// 512x256 texture, and drawing it at the screen's size restores its shape.
const FULL = "absolute top-0 left-0 w-[480] h-[272]";

/** A picture that fades in over whatever is under it. */
function FadeIn(props: { src: string }) {
  let ref: NodeMirror | undefined;
  onMount(() => {
    if (ref) animate(ref, "opacity", 1, { dur: 350, easing: "out" });
  });
  return (
    <Image
      ref={(el) => {
        ref = el;
      }}
      class={FULL}
      style={{ opacity: 0 }}
      src={props.src}
    />
  );
}

/**
 * The selected title's picture behind the screen, under a wash of the theme's
 * background so text stays readable. Each view leaves a different part clear:
 *
 *  - shelf: the top, behind the tiles;
 *  - grid: an even, darker wash, because tiles cover most of the screen;
 *  - list: the right side, beside the names.
 */
export function Backdrop(props: { state: LauncherState }) {
  const { state } = props;
  const theme = state.theme;
  return (
    <Show when={state.backdrop()}>
      <Show when={state.backdropUnder()} keyed>
        {(src) => <Image class={FULL} src={src} />}
      </Show>
      <Show when={state.backdrop()} keyed>
        {(src) => <FadeIn src={src} />}
      </Show>
      <Show when={state.view() === "carousel"}>
        <View
          class="absolute inset-0 bg-gradient-to-b from-black to-black"
          style={{ gradFrom: alpha(theme().bgTop, "73"), gradTo: alpha(theme().bgBottom, "f2") }}
        />
      </Show>
      <Show when={state.view() === "grid"}>
        <View
          class="absolute inset-0 bg-gradient-to-b from-black to-black"
          style={{ gradFrom: alpha(theme().bgTop, "c7"), gradTo: alpha(theme().bgBottom, "f5") }}
        />
      </Show>
      <Show when={state.view() === "list"}>
        <View
          class="absolute inset-0 bg-gradient-to-r from-black to-black"
          style={{ gradFrom: alpha(theme().bgBottom, "f5"), gradTo: alpha(theme().bgBottom, "33") }}
        />
        {/* The header and the footer keep their own wash across the full width. */}
        <View
          class="absolute top-0 left-0 right-0 h-[56] bg-gradient-to-b from-black to-black"
          style={{ gradFrom: alpha(theme().bgTop, "cc"), gradTo: alpha(theme().bgTop, "00") }}
        />
        <View
          class="absolute left-0 right-0 bottom-0 h-[90] bg-gradient-to-b from-black to-black"
          style={{ gradFrom: alpha(theme().bgBottom, "00"), gradTo: alpha(theme().bgBottom, "e6") }}
        />
      </Show>
    </Show>
  );
}
