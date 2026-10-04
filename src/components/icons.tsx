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
  // Status bar symbols, 12 wide in a 16 texture, in the ink for dark themes and
  // the "Day" ink for Daylight.
  bluetooth: { src: "icons/bluetooth.svg", box: "w-[12] h-[12] shrink-0 overflow-hidden", image: "w-[16] h-[16] shrink-0" },
  bluetoothDay: { src: "icons/bluetooth-day.svg", box: "w-[12] h-[12] shrink-0 overflow-hidden", image: "w-[16] h-[16] shrink-0" },
  wifi0: { src: "icons/wifi-0.svg", box: "w-[12] h-[12] shrink-0 overflow-hidden", image: "w-[16] h-[16] shrink-0" },
  wifi1: { src: "icons/wifi-1.svg", box: "w-[12] h-[12] shrink-0 overflow-hidden", image: "w-[16] h-[16] shrink-0" },
  wifi2: { src: "icons/wifi-2.svg", box: "w-[12] h-[12] shrink-0 overflow-hidden", image: "w-[16] h-[16] shrink-0" },
  wifi3: { src: "icons/wifi-3.svg", box: "w-[12] h-[12] shrink-0 overflow-hidden", image: "w-[16] h-[16] shrink-0" },
  wifi0Day: { src: "icons/wifi-0-day.svg", box: "w-[12] h-[12] shrink-0 overflow-hidden", image: "w-[16] h-[16] shrink-0" },
  wifi1Day: { src: "icons/wifi-1-day.svg", box: "w-[12] h-[12] shrink-0 overflow-hidden", image: "w-[16] h-[16] shrink-0" },
  wifi2Day: { src: "icons/wifi-2-day.svg", box: "w-[12] h-[12] shrink-0 overflow-hidden", image: "w-[16] h-[16] shrink-0" },
  wifi3Day: { src: "icons/wifi-3-day.svg", box: "w-[12] h-[12] shrink-0 overflow-hidden", image: "w-[16] h-[16] shrink-0" },
} as const;

export type IconName = keyof typeof ICONS;

export function Icon(props: { name: IconName }) {
  return (
    <View class={ICONS[props.name].box}>
      <Image class={ICONS[props.name].image} src={ICONS[props.name].src} />
    </View>
  );
}

/**
 * A piece of a prompt: words, an icon, the wider space between two hints, or
 * a place where the line may break with a normal space.
 */
export type Part = string | { icon: IconName } | { gap: true } | { wrap: true };

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

/**
 * A text with a `{button}` place as prompt parts: its words around the
 * button's icon. The line may break between any two of them.
 */
export function withButton(text: string, icon: IconName): Part[] {
  const parts: Part[] = [];
  for (const word of text.split(/\s+/).filter(Boolean)) {
    if (parts.length > 0) parts.push({ wrap: true });
    const [before, after] = word.split("{button}");
    if (after === undefined) {
      parts.push(word);
      continue;
    }
    // Punctuation written against the place stays next to the icon.
    if (before) parts.push(before);
    parts.push({ icon });
    if (after) parts.push(after);
  }
  return parts;
}

/** The parts between two breaks, and whether the wider space follows them. */
interface Group {
  parts: (string | { icon: IconName })[];
  spaced: boolean;
}

function groupsOf(parts: Part[]): Group[] {
  const groups: Group[] = [{ parts: [], spaced: false }];
  for (const part of parts) {
    const last = groups[groups.length - 1];
    if (typeof part === "object" && ("gap" in part || "wrap" in part)) {
      last.spaced = "gap" in part;
      groups.push({ parts: [], spaced: false });
    } else last.parts.push(part);
  }
  return groups.filter((group) => group.parts.length > 0);
}

/**
 * Dim text and button icons. A hint stays whole; hints that do not fit the
 * width move to the next line, since a translated label can be longer than
 * the English one the line was sized for. `center` centers each line in a
 * parent that gives the prompt a width.
 */
export function Prompt(props: { state: LauncherState; parts: Part[]; center?: boolean }) {
  return (
    <View
      class={
        props.center
          ? "flex-row flex-wrap items-center justify-center gap-1 shrink-0"
          : "flex-row flex-wrap items-center gap-1 shrink-0"
      }
    >
      <For each={groupsOf(props.parts)}>
        {(group) => (
          <View
            class={
              group.spaced
                ? "flex-row items-center gap-1 h-[16] shrink-0 pr-2"
                : "flex-row items-center gap-1 h-[16] shrink-0"
            }
          >
            <For each={group.parts}>
              {(part) => (
                <Switch>
                  <Match when={typeof part === "string"}>
                    <Text class={props.state.text().small} style={{ textColor: props.state.theme().dim }}>
                      {part as string}
                    </Text>
                  </Match>
                  <Match when={typeof part === "object" ? part.icon : undefined}>
                    {(name) => <Icon name={name()} />}
                  </Match>
                </Switch>
              )}
            </For>
          </View>
        )}
      </For>
    </View>
  );
}
