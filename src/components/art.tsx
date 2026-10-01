import { Show } from "solid-js";
import { Image, Text, View } from "@pocketjs/framework/components";
import type { LauncherState } from "../state.ts";
import type { Game } from "../types.ts";

// Class literals per size. They stay full literals because the compiler does
// not accept interpolated class fragments. Tiles are square: the engine draws
// textures as rectangles, and Vita icons fill their whole 128x128 frame.
const SIZES = {
  // Shelf tile, one size per detail level (see SHELF in layout.ts).
  shelfBasic: {
    box: "relative w-[124] h-[124] items-center justify-center",
    image: "w-[124] h-[124]",
    text: "text-4xl text-white font-bold",
  },
  shelfNormal: {
    box: "relative w-[112] h-[112] items-center justify-center",
    image: "w-[112] h-[112]",
    text: "text-4xl text-white font-bold",
  },
  shelfDetailed: {
    box: "relative w-[100] h-[100] items-center justify-center",
    image: "w-[100] h-[100]",
    text: "text-2xl text-white font-bold",
  },
  // List detail pane.
  lg: {
    box: "relative w-[104] h-[104] items-center justify-center",
    image: "w-[104] h-[104]",
    text: "text-4xl text-white font-bold",
  },
  // Grid tile.
  md: {
    box: "relative w-[72] h-[72] items-center justify-center",
    image: "w-[72] h-[72]",
    text: "text-xl text-white font-bold",
  },
  // List row.
  sm: {
    box: "relative w-[22] h-[22] shrink-0 items-center justify-center",
    image: "w-[22] h-[22]",
    text: "text-xs text-white font-bold",
  },
} as const;

// Fill behind the initials of a title with no icon, one per tint.
const TINT_ART = [
  "absolute inset-0 bg-gradient-to-b from-amber-400 to-red-600",
  "absolute inset-0 bg-gradient-to-b from-emerald-400 to-teal-700",
  "absolute inset-0 bg-gradient-to-b from-sky-400 to-indigo-700",
  "absolute inset-0 bg-gradient-to-b from-purple-400 to-violet-800",
  "absolute inset-0 bg-gradient-to-b from-pink-400 to-rose-700",
  "absolute inset-0 bg-gradient-to-b from-cyan-300 to-blue-700",
] as const;

/**
 * Wash of the selected title's tint over the top of the screen. The class
 * swaps when the selection moves and `transition-colors` blends the change.
 */
export const TINT_AMBIENT = [
  "absolute top-0 left-0 right-0 h-[190] bg-gradient-to-b from-[#f59e0b55] to-[#f59e0b00] transition-colors duration-300",
  "absolute top-0 left-0 right-0 h-[190] bg-gradient-to-b from-[#10b98155] to-[#10b98100] transition-colors duration-300",
  "absolute top-0 left-0 right-0 h-[190] bg-gradient-to-b from-[#3b82f655] to-[#3b82f600] transition-colors duration-300",
  "absolute top-0 left-0 right-0 h-[190] bg-gradient-to-b from-[#8b5cf655] to-[#8b5cf600] transition-colors duration-300",
  "absolute top-0 left-0 right-0 h-[190] bg-gradient-to-b from-[#ec489955] to-[#ec489900] transition-colors duration-300",
  "absolute top-0 left-0 right-0 h-[190] bg-gradient-to-b from-[#06b6d455] to-[#06b6d400] transition-colors duration-300",
] as const;

export type ArtSize = keyof typeof SIZES;

/** Up to two initials of a title, for tiles that have no icon. */
function initials(title: string): string {
  const words = title.split(" ").filter((word) => word.length > 0);
  const letters = words.length > 1 ? `${words[0][0]}${words[1][0]}` : title.slice(0, 2);
  return letters.toUpperCase();
}

/**
 * A title's picture: its icon once the host has decoded it, otherwise its
 * initials (one letter at the small size) over the title's tint. An icon with
 * transparent parts gets a box behind it in the icon's own color, when the
 * "Icon box" setting is on.
 */
export function TitleArt(props: { game: Game; size: ArtSize; state: LauncherState }) {
  // Read through a function: the shelf changes `size` when the detail level changes.
  const size = () => SIZES[props.size];
  const icon = () => props.state.icons()[props.game.id];
  const box = () => props.state.iconBox(props.game.id);
  const label = () => (props.size === "sm" ? props.game.title.slice(0, 1).toUpperCase() : initials(props.game.title));
  return (
    <View class={size().box}>
      <Show
        when={icon()}
        fallback={
          <>
            <View class={TINT_ART[props.game.tint % TINT_ART.length]} />
            <Text class={size().text}>{label()}</Text>
          </>
        }
      >
        <Show when={box()}>
          {(colors) => (
            <View
              class="absolute inset-0 bg-gradient-to-b from-black to-black"
              style={{ gradFrom: colors().from, gradTo: colors().to }}
            />
          )}
        </Show>
        <Image class={size().image} src={icon() ?? ""} />
      </Show>
    </View>
  );
}

/**
 * The frame around the selected tile: four bars, 3 px outside the art on each
 * side. Nothing is drawn behind the art, so a see-through icon shows the
 * screen (or its box) and not the frame's color.
 */
export function SelectedFrame(props: { state: LauncherState }) {
  const color = () => ({ bgColor: props.state.theme().accent });
  return (
    <>
      <View class="absolute left-[-3] right-[-3] top-[-3] h-[3]" style={color()} />
      <View class="absolute left-[-3] right-[-3] bottom-[-3] h-[3]" style={color()} />
      <View class="absolute left-[-3] top-0 bottom-0 w-[3]" style={color()} />
      <View class="absolute right-[-3] top-0 bottom-0 w-[3]" style={color()} />
    </>
  );
}
