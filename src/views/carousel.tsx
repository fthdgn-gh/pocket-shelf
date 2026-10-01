import { createEffect, createSignal, For, onMount, Show, untrack } from "solid-js";
import { View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { onFrame } from "@pocketjs/framework/lifecycle";
import { getOps, hostViewport } from "@pocketjs/framework/host";
import { SCREEN_H, SCREEN_W } from "../../contracts/spec/spec.ts";
import { TitleArt, type ArtSize } from "../components/art.tsx";
import { carouselLayout } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import { alpha } from "../themes.ts";
import type { TextRole } from "../text.ts";
import type { DetailLevel, Game } from "../types.ts";
import { SelectedInfo } from "./info.tsx";

// Class literals per detail level; the sizes match SHELF in layout.ts. A tile
// stands on the shelf: the selected one grows from its bottom edge and the
// rest sit smaller and dimmer beside it.
const SHAPES: Record<
  DetailLevel,
  { art: ArtSize; tile: string; selected: string; row: string; strip: string; role: TextRole }
> = {
  basic: {
    art: "shelfBasic",
    tile: "relative shrink-0 w-[124] h-[124] origin-bottom scale-90 opacity-60 transition duration-200 ease-out",
    selected: "relative shrink-0 w-[124] h-[124] origin-bottom scale-125 opacity-100 transition duration-200 ease-out",
    row: "flex-row items-end justify-start w-full h-[172] shrink-0 pb-[8] overflow-hidden",
    strip: "flex-row items-end gap-[26] shrink-0",
    role: "title",
  },
  normal: {
    art: "shelfNormal",
    tile: "relative shrink-0 w-[112] h-[112] origin-bottom scale-90 opacity-60 transition duration-200 ease-out",
    selected: "relative shrink-0 w-[112] h-[112] origin-bottom scale-125 opacity-100 transition duration-200 ease-out",
    row: "flex-row items-end justify-start w-full h-[156] shrink-0 pb-[8] overflow-hidden",
    strip: "flex-row items-end gap-[24] shrink-0",
    role: "title",
  },
  detailed: {
    art: "shelfDetailed",
    tile: "relative shrink-0 w-[100] h-[100] origin-bottom scale-90 opacity-60 transition duration-200 ease-out",
    selected: "relative shrink-0 w-[100] h-[100] origin-bottom scale-125 opacity-100 transition duration-200 ease-out",
    row: "flex-row items-end justify-start w-full h-[140] shrink-0 pb-[8] overflow-hidden",
    strip: "flex-row items-end gap-[22] shrink-0",
    role: "heading",
  },
};

function ShelfTile(props: { game: Game; state: LauncherState; selected: boolean }) {
  const shape = () => SHAPES[props.state.detail()];
  return (
    <View class={props.selected ? shape().selected : shape().tile}>
      {/* The selected tile's frame: a filled square behind the art, 3 px larger on each side. */}
      <Show when={props.selected}>
        <View class="absolute inset-[-3]" style={{ bgColor: props.state.theme().accent }} />
      </Show>
      <TitleArt size={shape().art} game={props.game} icon={props.state.icons()[props.game.id]} />
    </View>
  );
}

/** One row of tiles on a shelf; the row slides so the selected tile sits at the center. */
export function CarouselView(props: { state: LauncherState }) {
  const { state } = props;
  let stripRef: NodeMirror | undefined;
  let rowRef: NodeMirror | undefined;

  // Viewport resolution tracked from the host, which desktop and browser hosts resize.
  const readViewport = () => hostViewport(getOps()) ?? { w: SCREEN_W, h: SCREEN_H };
  const [viewport, setViewport] = createSignal(readViewport());
  const stripX = (index: number) => {
    const layout = carouselLayout(viewport().w, state.detail());
    return layout.centerX - index * layout.cardPitch;
  };

  onMount(() => {
    onFrame(() => {
      const current = readViewport();
      const previous = viewport();
      if (current.w !== previous.w || current.h !== previous.h) setViewport(current);
    });
    // The row rises onto the shelf when the view or the category appears.
    if (rowRef) {
      animate(rowRef, "translateY", 0, { dur: 220, easing: "out" });
      animate(rowRef, "opacity", 1, { dur: 220, easing: "out" });
    }
  });

  createEffect(() => {
    state.games(); // re-run when the category changes the list
    const index = state.selectedIndex();
    if (stripRef) animate(stripRef, "translateX", stripX(index), { dur: 220, easing: "out" });
  });

  return (
    <View class="flex-col w-full grow">
      <View
        ref={(el) => {
          rowRef = el;
        }}
        class={SHAPES[state.detail()].row}
        style={{ translateY: 14, opacity: 0 }}
      >
        <View
          ref={(el) => {
            stripRef = el;
          }}
          class={SHAPES[state.detail()].strip}
          style={{ translateX: untrack(() => stripX(state.selectedIndex())) }}
        >
          <For each={state.games()}>
            {(game, index) => (
              <ShelfTile
                game={game}
                state={state}
                selected={state.selectedIndex() === index()}
              />
            )}
          </For>
        </View>
      </View>
      {/* The shelf: a lit edge, and its glow falling onto the surface below. */}
      <View class="relative flex-col items-center w-full grow">
        <View class="absolute top-0 left-0 right-0 h-[2]" style={{ bgColor: alpha(state.theme().accent, "99") }} />
        <View
          class="absolute top-[2] left-0 right-0 h-[44] bg-gradient-to-b from-black to-black"
          style={{ gradFrom: alpha(state.theme().accent, "2e"), gradTo: alpha(state.theme().accent, "00") }}
        />
        <View class="h-[10] shrink-0" />
        <SelectedInfo
          state={state}
          game={state.games()[state.selectedIndex()]}
          role={SHAPES[state.detail()].role}
          width={440}
        />
      </View>
    </View>
  );
}
