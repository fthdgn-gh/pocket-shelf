import { createEffect, For, Show, untrack } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { getOps, hostViewport } from "@pocketjs/framework/host";
import { fitTitle } from "../catalog.ts";
import { fontSlot } from "../text.ts";
import { SCREEN_W } from "../../contracts/spec/spec.ts";
import { HEADER_SIDE, TAB_GAP, TAB_W } from "../layout.ts";
import type { LauncherState } from "../state.ts";

/** A shoulder-button badge at one end of the header. */
function Shoulder(props: { state: LauncherState; label: string }) {
  return (
    <View class="w-[24] h-[16] shrink-0 items-center justify-center rounded-[5] border" style={{ borderColor: props.state.theme().line }}>
      <Text class={props.state.text().smallBold} style={{ textColor: props.state.theme().dim }}>
        {props.label}
      </Text>
    </View>
  );
}

/**
 * Category header: L and R badges on the sides, the category names between
 * them. The strip slides so the current category sits in the center, marked
 * by a bar under its name.
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
          <Shoulder state={state} label="L" />
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
              return (
                <View class="flex-col items-center justify-center gap-[3] w-[76] h-[30] shrink-0">
                  <Text
                    class={active() ? state.text().bodyBold : state.text().body}
                    style={{ textColor: active() ? state.theme().text : state.theme().dim }}
                  >
                    {fitTitle(item.label, 74, fontSlot(state.font(), active() ? "bodyBold" : "body"))}
                  </Text>
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
          <Shoulder state={state} label="R" />
        </Show>
      </View>
    </View>
  );
}
