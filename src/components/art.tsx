import { Show } from "solid-js";
import { Image, Text, View } from "@pocketjs/framework/components";
import type { Game } from "../types.ts";

// Class literals per size. They stay full literals because the compiler does
// not accept interpolated class fragments.
const SIZES = {
  lg: {
    box: "relative w-full h-[90] rounded-lg items-center justify-center",
    image: "w-[84] h-[84]",
    text: "text-xl text-white font-bold",
  },
  md: {
    box: "relative w-full h-[48] rounded-lg items-center justify-center",
    image: "w-[44] h-[44]",
    text: "text-xs text-white font-bold",
  },
  sm: {
    box: "relative w-[28] h-[28] shrink-0 rounded-lg items-center justify-center",
    image: "w-[28] h-[28]",
    text: "text-xs text-white font-bold",
  },
} as const;

export type ArtSize = keyof typeof SIZES;

/**
 * A title's picture: its icon once the host has decoded it, otherwise its id
 * (or first letter at the small size) over the title's gradient.
 */
export function TitleArt(props: { game: Game; size: ArtSize; icon: string | undefined }) {
  const size = SIZES[props.size];
  const label = () => (props.size === "sm" ? props.game.title.slice(0, 1).toUpperCase() : props.game.id);
  return (
    <View class={size.box}>
      <View class={props.game.gradient} />
      <Show when={props.icon} fallback={<Text class={size.text}>{label()}</Text>}>
        <Image class={size.image} src={props.icon ?? ""} />
      </Show>
    </View>
  );
}
