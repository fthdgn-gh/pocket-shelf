import { createEffect, createSignal, For, onMount, Show, untrack } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { focusNode } from "@pocketjs/framework/input";
import { onFrame } from "@pocketjs/framework/lifecycle";
import { getOps, hostViewport } from "@pocketjs/framework/host";
import { SCREEN_H, SCREEN_W } from "../../contracts/spec/spec.ts";
import { fitTitle } from "../catalog.ts";
import { fontSlot } from "../fonts.ts";
import { TitleArt } from "../components/art.tsx";
import { carouselLayout } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import type { Game } from "../types.ts";

function GameCard(props: {
  game: Game;
  state: LauncherState;
  onPress: () => void;
  ref: (node: NodeMirror) => void;
}) {
  const theme = props.state.theme;
  return (
    <View ref={props.ref} class={theme().card} focusable onPress={props.onPress}>
      <TitleArt size="lg" game={props.game} icon={props.state.icons()[props.game.id]} />
      <Show when={props.state.detail() !== "basic"}>
        <View class="flex-col mt-2 px-1 w-full overflow-hidden">
          <Text class={theme().cardTitle}>{fitTitle(props.game.title, 138, fontSlot(props.state.font(), 12, true))}</Text>
          <Show when={props.state.detail() === "detailed"}>
            <Text class={theme().cardMeta}>{props.game.id}</Text>
          </Show>
        </View>
      </Show>
    </View>
  );
}

/** One row of cards; the strip slides so the selected card sits at the center. */
export function CarouselView(props: { state: LauncherState }) {
  const { state } = props;
  const cardRefs: (NodeMirror | undefined)[] = [];
  let stripRef: NodeMirror | undefined;

  // Viewport resolution tracked from the host, which desktop and browser hosts resize.
  const readViewport = () => hostViewport(getOps()) ?? { w: SCREEN_W, h: SCREEN_H };
  const [viewport, setViewport] = createSignal(readViewport());
  const stripX = (index: number) => {
    const layout = carouselLayout(viewport().w);
    return layout.centerX - index * layout.cardPitch;
  };

  onMount(() => {
    onFrame(() => {
      const current = readViewport();
      const previous = viewport();
      if (current.w !== previous.w || current.h !== previous.h) setViewport(current);
    });
  });

  createEffect(() => {
    state.games(); // re-run when the category changes the list
    const index = state.selectedIndex();
    const node = cardRefs[index];
    if (node) focusNode(node);
    if (stripRef) animate(stripRef, "translateX", stripX(index), { dur: 200, easing: "out" });
  });

  return (
    <View class="w-full grow overflow-hidden items-center justify-start">
      <View
        ref={(el) => {
          stripRef = el;
        }}
        class="flex-row items-center gap-4"
        style={{ translateX: untrack(() => stripX(state.selectedIndex())) }}
      >
        <For each={state.games()}>
          {(game, index) => (
            <GameCard
              game={game}
              state={state}
              onPress={() => state.activate(index())}
              ref={(el) => {
                cardRefs[index()] = el;
              }}
            />
          )}
        </For>
      </View>
    </View>
  );
}
