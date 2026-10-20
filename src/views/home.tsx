import { createEffect, For, on, onMount, Show, untrack } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { SelectedFrame, TileShadow, TitleArt, type ArtSize } from "../components/art.tsx";
import { Icon } from "../components/icons.tsx";
import { fitTitle, shownId } from "../catalog.ts";
import { HOME_AFTER, HOME_LEFT, homeSlot } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import { fontSlot } from "../text.ts";
import type { DetailLevel, Game } from "../types.ts";
import { createWindow } from "./window.ts";

// Class literals per detail level; the sizes match HOME in layout.ts. Every
// tile sits at the row's left edge at the selected tile's size and is moved
// and scaled by `homeSlot`. Transitions only follow class changes, so the
// tiles are tweened with `animate` when the selection moves.
const SHAPES: Record<DetailLevel, { art: ArtSize; tile: string; row: string }> = {
  basic: {
    art: "homeBasic",
    tile: "absolute left-0 top-[8] w-[128] h-[128] origin-top-left",
    row: "relative w-full h-[144] shrink-0",
  },
  normal: {
    art: "homeNormal",
    tile: "absolute left-0 top-[8] w-[96] h-[96] origin-top-left",
    row: "relative w-full h-[112] shrink-0",
  },
  detailed: {
    art: "homeDetailed",
    tile: "absolute left-0 top-[8] w-[88] h-[88] origin-top-left",
    row: "relative w-full h-[104] shrink-0",
  },
};

const MOVE = { dur: 200, easing: "out" } as const;
/** Text room under the row: the screen less the row's left margin on both sides. */
const TEXT_W = 480 - HOME_LEFT * 2;

function HomeTile(props: { game: Game; state: LauncherState; offset: number }) {
  let tileRef: NodeMirror | undefined;
  const slot = () => homeSlot(props.state.detail(), props.offset);
  // A tile mounts in its place; only later moves are tweened.
  const first = untrack(slot);

  createEffect(
    on(
      slot,
      (next) => {
        if (!tileRef) return;
        animate(tileRef, "translateX", next.x, MOVE);
        animate(tileRef, "scale", next.scale, MOVE);
        animate(tileRef, "opacity", next.opacity, MOVE);
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
      style={{ translateX: first.x, scale: first.scale, opacity: first.opacity, zIndex: 100 - Math.abs(props.offset) }}
    >
      <Show when={props.offset === 0}>
        <TileShadow />
        <SelectedFrame state={props.state} />
      </Show>
      <TitleArt size={SHAPES[props.state.detail()].art} game={props.game} state={props.state} />
    </View>
  );
}

/**
 * The selected title's name, a line about it, and a button that starts it,
 * from the row's left edge. "Basic" shows none of it; "Detailed" adds the
 * title id to the line.
 */
function HomeInfo(props: { state: LauncherState; game: Game | undefined }) {
  const { state } = props;
  const theme = state.theme;
  const line = (game: Game) => {
    const parts = [game.genre, state.categoryLabel(game.category)];
    if (state.detail() === "detailed") parts.push(shownId(game));
    return parts.filter((part, index) => index === 0 || part !== parts[0]).join("  ·  ");
  };
  return (
    <Show when={state.detail() !== "basic" ? props.game : undefined}>
      {(game) => (
        <View class="flex-col items-start gap-[4]" style={{ paddingL: HOME_LEFT }}>
          <Text class={state.text().display} style={{ textColor: theme().text }}>
            {fitTitle(game().title, TEXT_W, fontSlot(state.font(), "display"))}
          </Text>
          <Text class={state.text().small} style={{ textColor: theme().dim }}>
            {fitTitle(line(game()), TEXT_W, fontSlot(state.font(), "small"))}
          </Text>
          <View class="h-[4] shrink-0" />
          <View
            class="flex-row items-center gap-[6] h-[26] pl-[6] pr-[14] rounded-[13] shrink-0"
            style={{ bgColor: theme().text }}
          >
            <Icon name={state.confirmButton()} />
            <Text class={state.text().bodyBold} style={{ textColor: theme().bgBottom }}>
              {state.t().launch}
            </Text>
          </View>
        </View>
      )}
    </Show>
  );
}

/**
 * The home row, after the PS5 home screen: the selected title large at the
 * left, the next ones smaller after it, and its name and a start button
 * below. A move slides the row: the selected tile leaves to the left and the
 * next one grows into its place.
 */
export function HomeView(props: { state: LauncherState }) {
  const { state } = props;
  let rowRef: NodeMirror | undefined;
  // One tile before the selection stays mounted so it can slide out.
  const shelf = createWindow(state, () => [state.selectedIndex() - 1, state.selectedIndex() + HOME_AFTER + 1]);

  onMount(() => {
    // The row slides in from the right when the view or the category appears.
    if (rowRef) {
      animate(rowRef, "translateX", 0, { dur: 240, easing: "out" });
      animate(rowRef, "opacity", 1, { dur: 240, easing: "out" });
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
          {(game) => <HomeTile game={game} state={state} offset={shelf.indexOf(game) - state.selectedIndex()} />}
        </For>
      </View>
      <View class="h-[6] shrink-0" />
      <HomeInfo state={state} game={state.games()[state.selectedIndex()]} />
    </View>
  );
}
