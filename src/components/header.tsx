import { createEffect, For, Show, untrack } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { getOps, hostViewport } from "@pocketjs/framework/host";
import { fitTitle } from "../catalog.ts";
import { fontSlot } from "../text.ts";
import { SCREEN_W } from "../../contracts/spec/spec.ts";
import { HEADER_SIDE, TAB_COUNT_GAP, TAB_COUNT_PAD, TAB_GAP, TAB_W } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import { alpha } from "../themes.ts";
import { Icon } from "./icons.tsx";

/**
 * Category header: the L and R buttons on the sides, the category names between
 * them, each followed by its number of titles. The strip slides so the current
 * category sits in the center, marked by a bar under its name.
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
    if (stripRef) animate(stripRef, "translateX", x, { dur: 220, easing: "out" });
  });

  const many = () => state.categories().length > 1;

  return (
    <View class="flex-row items-center gap-[6] w-full h-[34] shrink-0 px-3">
      <View class="w-[24] h-[16] shrink-0">
        <Show when={many()}>
          <Icon name="l" />
        </Show>
      </View>
      <View class="flex-row grow h-[34] overflow-hidden justify-start items-center">
        <View
          ref={(el) => {
            stripRef = el;
          }}
          class="flex-row items-center gap-1 shrink-0"
          style={{ translateX: untrack(stripX) }}
        >
          <For each={state.categories()}>
            {(item) => {
              const active = () => state.categoryId() === item.id;
              const count = () => String(state.categoryCount(item.id));
              // The name gets what the count's badge leaves of the tab.
              const labelW = () =>
                TAB_W -
                2 -
                TAB_COUNT_GAP -
                TAB_COUNT_PAD * 2 -
                getOps().measureText(count(), fontSlot(state.font(), "smallBold"));
              return (
                <View class="flex-col items-center justify-center gap-[3] w-[112] h-[30] shrink-0">
                  <View class="flex-row items-center justify-center gap-1">
                    <Text
                      class={active() ? state.text().bodyBold : state.text().body}
                      style={{ textColor: active() ? state.theme().text : state.theme().dim }}
                    >
                      {fitTitle(item.label, labelW(), fontSlot(state.font(), active() ? "bodyBold" : "body"))}
                    </Text>
                    <View
                      class="flex-row items-center justify-center h-[14] px-1 rounded-[7] shrink-0"
                      style={{ bgColor: active() ? alpha(state.theme().accent, "30") : state.theme().line }}
                    >
                      <Text
                        class={state.text().smallBold}
                        style={{ textColor: active() ? state.theme().accent : state.theme().dim }}
                      >
                        {count()}
                      </Text>
                    </View>
                  </View>
                  <View
                    class="w-[20] h-[2] rounded-[1]"
                    style={{ bgColor: active() ? state.theme().accent : "#00000000" }}
                  />
                </View>
              );
            }}
          </For>
        </View>
      </View>
      <View class="w-[24] h-[16] shrink-0">
        <Show when={many()}>
          <Icon name="r" />
        </Show>
      </View>
    </View>
  );
}
