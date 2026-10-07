import { createEffect, For, on, onMount, Show, untrack } from "solid-js";
import { View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { SelectedFrame, TitleArt, type ArtSize } from "../components/art.tsx";
import { CASCADE_DEPTH, CASCADE_LEFT, cascadeSlot } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import type { TextRole } from "../text.ts";
import type { DetailLevel, Game } from "../types.ts";
import { SelectedInfo } from "./info.tsx";
import { createWindow } from "./window.ts";

// Class literals per detail level; the sizes match CASCADE in layout.ts. Every
// tile sits at the row's left edge, vertically centered, and is moved, scaled
// and shaded by `cascadeSlot`. Transitions only follow class changes, so the
// tiles are tweened with `animate` when the selection moves.

const SHAPES: Record<DetailLevel, { art: ArtSize; tile: string; row: string; role: TextRole }> = {
  basic: {
    art: "cascadeBasic",
    tile: "absolute left-0 top-[12] w-[144] h-[144] origin-left",
    row: "relative w-full h-[168] shrink-0 overflow-hidden",
    role: "title",
  },
  normal: {
    art: "cascadeNormal",
    tile: "absolute left-0 top-[12] w-[128] h-[128] origin-left",
    row: "relative w-full h-[152] shrink-0 overflow-hidden",
    role: "title",
  },
  detailed: {
    art: "cascadeDetailed",
    tile: "absolute left-0 top-[12] w-[112] h-[112] origin-left",
    row: "relative w-full h-[136] shrink-0 overflow-hidden",
    role: "heading",
  },
};

const MOVE = { dur: 240, easing: "out" } as const;

function CascadeTile(props: { game: Game; state: LauncherState; offset: number }) {
  let tileRef: NodeMirror | undefined;
  let shadeRef: NodeMirror | undefined;
  const slot = () => cascadeSlot(props.state.detail(), props.offset);
  // A tile mounts in its place; only later moves are tweened.
  const first = untrack(slot);

  createEffect(
    on(
      slot,
      (next) => {
        if (tileRef) {
          animate(tileRef, "translateX", next.x, MOVE);
          animate(tileRef, "translateY", next.y, MOVE);
          animate(tileRef, "scale", next.scale, MOVE);
          animate(tileRef, "opacity", next.opacity, MOVE);
        }
        if (shadeRef) animate(shadeRef, "opacity", next.shade, MOVE);
      },
      { defer: true },
    ),
  );

  return (
    <View
      ref={(el) => {
        tileRef = el;
      }}
      class={SHAPES[props.state.detail()].tile}
      style={{
        translateX: first.x,
        translateY: first.y,
        scale: first.scale,
        opacity: first.opacity,
        zIndex: slot().z,
      }}
    >
      <Show when={props.offset === 0}>
        <SelectedFrame state={props.state} />
      </Show>
      <TitleArt size={SHAPES[props.state.detail()].art} game={props.game} state={props.state} />
      {/* Darkens the tiles behind the selected one; see-through would show the tiles under it. */}
      <View
        ref={(el) => {
          shadeRef = el;
        }}
        class="absolute inset-0"
        style={{ bgColor: props.state.theme().bgBottom, opacity: first.shade }}
      />
    </View>
  );
}

/**
 * The selected title at the left at full size and the next ones receding to the
 * right, after the Xbox 360 dashboard. A move slides the row: the selected
 * tile leaves to the left and the next one grows into its place.
 */
export function CascadeView(props: { state: LauncherState }) {
  const { state } = props;
  let rowRef: NodeMirror | undefined;
  // One tile before the selection stays mounted so it can slide out.
  const shelf = createWindow(state, () => [state.selectedIndex() - 1, state.selectedIndex() + CASCADE_DEPTH + 2]);

  onMount(() => {
    // The row slides in from the right when the view or the category appears.
    if (rowRef) {
      animate(rowRef, "translateX", 0, { dur: 260, easing: "out" });
      animate(rowRef, "opacity", 1, { dur: 260, easing: "out" });
    }
  });

  return (
    <View class="flex-col w-full grow">
      <View
        ref={(el) => {
          rowRef = el;
        }}
        class={SHAPES[state.detail()].row}
        style={{ translateX: 24, opacity: 0 }}
      >
        <For each={shelf.items()}>
          {(game) => (
            <CascadeTile game={game} state={state} offset={shelf.indexOf(game) - state.selectedIndex()} />
          )}
        </For>
      </View>
      <View class="flex-col w-full grow" style={{ paddingL: CASCADE_LEFT }}>
        <SelectedInfo
          state={state}
          game={state.games()[state.selectedIndex()]}
          role={SHAPES[state.detail()].role}
          width={480 - CASCADE_LEFT * 2}
          start
        />
      </View>
    </View>
  );
}
