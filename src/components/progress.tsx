import { View } from "@pocketjs/framework/components";
import type { Theme } from "../themes.ts";

/**
 * A horizontal bar filled from the left: `done` of `total`. With no total yet
 * it shows empty.
 */
export function ProgressBar(props: { theme: Theme; width: number; done: number; total: number }) {
  const filled = () => (props.total > 0 ? Math.round((props.width * Math.min(props.done, props.total)) / props.total) : 0);
  return (
    <View class="relative h-[6] rounded-[3] shrink-0" style={{ width: props.width, bgColor: props.theme.line }}>
      <View class="absolute left-0 top-0 h-[6] rounded-[3]" style={{ width: filled(), bgColor: props.theme.accent }} />
    </View>
  );
}
