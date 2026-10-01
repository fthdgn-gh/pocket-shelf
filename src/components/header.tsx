import { createEffect, For, untrack } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { getOps, hostViewport } from "@pocketjs/framework/host";
import { fitTitle } from "../catalog.ts";
import { fontSlot } from "../fonts.ts";
import { SCREEN_W } from "../../contracts/spec/spec.ts";
import { HEADER_SIDE, TAB_GAP, TAB_W } from "../layout.ts";
import type { LauncherState } from "../state.ts";

/**
 * Category header: L and R badges on the sides, the category tabs between
 * them. The strip slides so the current category sits in the center.
 */
export function Header(props: { state: LauncherState }) {
  const { state } = props;
  let stripRef: NodeMirror | undefined;

  const viewportW = untrack(() => hostViewport(getOps())?.w ?? SCREEN_W);
  const stripX = () => {
    const index = Math.max(0, state.categories().findIndex((item) => item.id === state.categoryId()));
    const center = (viewportW - HEADER_SIDE * 2) / 2;
    return center - TAB_W / 2 - index * (TAB_W + TAB_GAP);
  };

  createEffect(() => {
    const x = stripX();
    if (stripRef) animate(stripRef, "translateX", x, { dur: 200, easing: "out" });
  });

  return (
    <View class="flex-row items-center gap-2 w-full h-[28] px-5 mb-3">
      <View class={state.theme().pill}>
        <Text class={state.theme().pillText}>L</Text>
      </View>
      <View class="grow h-[28] overflow-hidden justify-start items-center">
        <View
          ref={(el) => {
            stripRef = el;
          }}
          class="flex-row items-center gap-2"
          style={{ translateX: untrack(stripX) }}
        >
          <For each={state.categories()}>
            {(item) => {
              const active = () => state.categoryId() === item.id;
              return (
                <View class={active() ? state.theme().tabActive : state.theme().tab}>
                  <Text class={active() ? state.theme().tabTextActive : state.theme().tabText}>
                    {fitTitle(item.label, 84, fontSlot(state.font(), 12, active()))}
                  </Text>
                </View>
              );
            }}
          </For>
        </View>
      </View>
      <View class={state.theme().pill}>
        <Text class={state.theme().pillText}>R</Text>
      </View>
    </View>
  );
}
