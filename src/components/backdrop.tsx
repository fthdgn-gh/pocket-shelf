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
 * background so text stays readable:
 *
 *  - shelf and cascade: clearest at the top, behind the tiles;
 *  - grid and list: an even, darker wash, because tiles and rows cover the screen.
 */
export function Backdrop(props: { state: LauncherState }) {
  const { state } = props;
  const theme = state.theme;
  // Views whose tiles leave most of the screen uncovered.
  const open = () => state.view() === "carousel" || state.view() === "cascade";
  return (
    <Show when={state.backdrop()}>
      <Show when={state.backdropUnder()} keyed>
        {(src) => <Image class={FULL} src={src} />}
      </Show>
      <Show when={state.backdrop()} keyed>
        {(src) => <FadeIn src={src} />}
      </Show>
      <Show when={open()}>
        <View
          class="absolute inset-0 bg-gradient-to-b from-black to-black"
          style={{ gradFrom: alpha(theme().bgTop, "73"), gradTo: alpha(theme().bgBottom, "f2") }}
        />
      </Show>
      <Show when={!open()}>
        <View
          class="absolute inset-0 bg-gradient-to-b from-black to-black"
          style={{ gradFrom: alpha(theme().bgTop, "c7"), gradTo: alpha(theme().bgBottom, "f5") }}
        />
      </Show>
    </Show>
  );
}
