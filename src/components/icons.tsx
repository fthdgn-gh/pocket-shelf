import { For, Match, Switch } from "solid-js";
import { Image, Text, View } from "@pocketjs/framework/components";
import type { LauncherState } from "../state.ts";

// Icons drawn by `bun run shelf:icons` into src/icons/. Each entry names the
// SVG, the size it is laid out at, and the size of its texture. A texture's
// sides are powers of two, so an icon narrower than its texture sits at the
// left edge and `box` clips the rest.
const ICONS = {
  circle: { src: "icons/btn-circle.svg", box: "w-[16] h-[16] shrink-0", image: "w-[16] h-[16] shrink-0" },
  cross: { src: "icons/btn-cross.svg", box: "w-[16] h-[16] shrink-0", image: "w-[16] h-[16] shrink-0" },
  triangle: { src: "icons/btn-triangle.svg", box: "w-[16] h-[16] shrink-0", image: "w-[16] h-[16] shrink-0" },
  square: { src: "icons/btn-square.svg", box: "w-[16] h-[16] shrink-0", image: "w-[16] h-[16] shrink-0" },
  l: { src: "icons/btn-l.svg", box: "w-[24] h-[16] shrink-0 overflow-hidden", image: "w-[32] h-[16] shrink-0" },
  r: { src: "icons/btn-r.svg", box: "w-[24] h-[16] shrink-0 overflow-hidden", image: "w-[32] h-[16] shrink-0" },
  select: { src: "icons/btn-select.svg", box: "w-[40] h-[16] shrink-0 overflow-hidden", image: "w-[64] h-[16] shrink-0" },
  start: { src: "icons/btn-start.svg", box: "w-[36] h-[16] shrink-0 overflow-hidden", image: "w-[64] h-[16] shrink-0" },
  dpadVertical: { src: "icons/dpad-vertical.svg", box: "w-[16] h-[16] shrink-0", image: "w-[16] h-[16] shrink-0" },
  dpadHorizontal: {
    src: "icons/dpad-horizontal.svg",
    box: "w-[16] h-[16] shrink-0",
    image: "w-[16] h-[16] shrink-0",
  },
  arrowLeft: { src: "icons/arrow-left.svg", box: "w-[8] h-[16] shrink-0", image: "w-[8] h-[16] shrink-0" },
  arrowRight: { src: "icons/arrow-right.svg", box: "w-[8] h-[16] shrink-0", image: "w-[8] h-[16] shrink-0" },
  plus: { src: "icons/plus.svg", box: "w-[16] h-[16] shrink-0", image: "w-[16] h-[16] shrink-0" },
} as const;

export type IconName = keyof typeof ICONS;

export function Icon(props: { name: IconName }) {
  return (
    <View class={ICONS[props.name].box}>
      <Image class={ICONS[props.name].image} src={ICONS[props.name].src} />
    </View>
  );
}

/** A piece of a prompt: words, an icon, or the wider space between two hints. */
export type Part = string | { icon: IconName } | { gap: true };

/**
 * Button hints as prompt parts: each hint is its button (or buttons) followed
 * by what it does, with a wider space between hints.
 */
export function hints(...items: [buttons: IconName | IconName[], label: string][]): Part[] {
  const parts: Part[] = [];
  for (const [index, [buttons, label]] of items.entries()) {
    if (index > 0) parts.push({ gap: true });
    for (const icon of Array.isArray(buttons) ? buttons : [buttons]) parts.push({ icon });
    parts.push(label);
  }
  return parts;
}

/** One line of dim text and button icons. */
export function Prompt(props: { state: LauncherState; parts: Part[] }) {
  return (
    <View class="flex-row items-center gap-1 h-[16] shrink-0">
      <For each={props.parts}>
        {(part) => (
          <Switch>
            <Match when={typeof part === "string"}>
              <Text class={props.state.text().small} style={{ textColor: props.state.theme().dim }}>
                {part as string}
              </Text>
            </Match>
            <Match when={typeof part === "object" && "icon" in part ? part.icon : undefined}>
              {(name) => <Icon name={name()} />}
            </Match>
            <Match when={typeof part === "object" && "gap" in part}>
              <View class="w-[4] h-[16] shrink-0" />
            </Match>
          </Switch>
        )}
      </For>
    </View>
  );
}
