import { createEffect, createMemo, For, on, Show, untrack } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { getOps, hostViewport } from "@pocketjs/framework/host";
import { fitTitle } from "../catalog.ts";
import { fontSlot } from "../text.ts";
import { SCREEN_W } from "../../contracts/spec/spec.ts";
import { BAR_SIDE, PILL_ROOM, STATUS_W, TAB_GAP, TAB_MAX } from "../layout.ts";
import type { LauncherState } from "../state.ts";
import { Icon } from "./icons.tsx";
import { StatusItems } from "./status-bar.tsx";

/**
 * The top bar: the L and R buttons around the category names, the current
 * one in bold over a short accent bar, then the status items at the right.
 * The names slide so the current one sits in the middle between L and R, the
 * same way at every press. A name an end of the bar cuts shows in part and
 * faint, so the next ones can be glimpsed. A gradient over the ends could not
 * match what is behind the bar (a title's backdrop under the theme's wash)
 * and showed as dark blocks.
 */
/** The most a cut name shows at: half of it in view shows at half of this. */
const CUT_OPACITY = 0.5;

export function Header(props: { state: LauncherState }) {
  const { state } = props;
  let stripRef: NodeMirror | undefined;

  const viewportW = untrack(() => hostViewport(getOps())?.w ?? SCREEN_W);
  // The names are measured in bold, so a name keeps its width when it becomes current.
  const slot = () => fontSlot(state.font(), "bodyBold");
  const labels = createMemo(() => state.categories().map((item) => fitTitle(item.label, TAB_MAX, slot())));
  const widths = createMemo(() => labels().map((label) => getOps().measureText(label, slot())));
  const lefts = createMemo(() => {
    let x = 0;
    return widths().map((width) => {
      const left = x;
      x += width + TAB_GAP;
      return left;
    });
  });
  // The room for the names: the bar less its padding, L and R, and the status items.
  const room = () => viewportW - BAR_SIDE * 2 - PILL_ROOM * 2 - (state.statusBarOn() ? STATUS_W : 0);
  const scroll = createMemo(() => {
    const index = Math.max(0, state.categories().findIndex((item) => item.id === state.categoryId()));
    return (lefts()[index] ?? 0) + (widths()[index] ?? 0) / 2 - room() / 2;
  });

  createEffect(() => {
    const x = -scroll();
    if (stripRef) animate(stripRef, "translateX", x, { dur: 220, easing: "out" });
  });

  const many = () => state.categories().length > 1;

  return (
    <View class="flex-row items-center w-full h-[30] shrink-0 px-4 gap-[6]">
      <View class="w-[24] h-[16] shrink-0">
        <Show when={many()}>
          <Icon name="l" />
        </Show>
      </View>
      <View class="relative grow h-[30] overflow-hidden">
        <View
          ref={(el) => {
            stripRef = el;
          }}
          class="absolute left-0 top-0 h-[30]"
          style={{ translateX: -untrack(scroll) }}
        >
          <For each={state.categories()}>
            {(item, index) => {
              const active = () => state.categoryId() === item.id;
              let nameRef: NodeMirror | undefined;
              // A name cut by an end of the bar shows faint, fainter the less of
              // it shows; a whole one at full strength. Tweened with the slide.
              const opacity = () => {
                const left = (lefts()[index()] ?? 0) - scroll();
                const width = widths()[index()] ?? 1;
                const seen = Math.min(left + width, room()) - Math.max(left, 0);
                const part = Math.max(0, Math.min(1, seen / width));
                return part >= 1 ? 1 : part * CUT_OPACITY;
              };
              createEffect(
                on(
                  opacity,
                  (next) => {
                    if (nameRef) animate(nameRef, "opacity", next, { dur: 220, easing: "out" });
                  },
                  { defer: true },
                ),
              );
              return (
                <View
                  ref={(el) => {
                    nameRef = el;
                  }}
                  class="absolute top-0 h-[30] flex-col items-center justify-center gap-[3]"
                  style={{ insetL: lefts()[index()] ?? 0, width: widths()[index()] ?? 0, opacity: untrack(opacity) }}
                >
                  <Text
                    class={active() ? state.text().bodyBold : state.text().body}
                    style={{ textColor: active() ? state.theme().text : state.theme().dim }}
                  >
                    {labels()[index()] ?? ""}
                  </Text>
                  <View
                    class="w-[14] h-[2] rounded-[1]"
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
      <Show when={state.statusBarOn()}>
        <View class="flex-row justify-end shrink-0" style={{ width: STATUS_W - 6 }}>
          <StatusItems state={state} />
        </View>
      </Show>
    </View>
  );
}
