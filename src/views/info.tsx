import { Show } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import { fitTitle } from "../catalog.ts";
import type { LauncherState } from "../state.ts";
import { fontSlot, type TextRole } from "../text.ts";
import type { Game } from "../types.ts";

/**
 * Name and details of the selected title, under the shelf and the grid.
 * "Basic" shows nothing, "Normal" the title, "Detailed" adds its id and category.
 */
export function SelectedInfo(props: { state: LauncherState; game: Game | undefined; role: TextRole; width: number }) {
  const { state } = props;
  return (
    <Show when={state.detail() !== "basic" ? props.game : undefined}>
      {(game) => (
        <View class="flex-col items-center gap-[2]">
          <Text class={state.text()[props.role]} style={{ textColor: state.theme().text }}>
            {fitTitle(game().title, props.width, fontSlot(state.font(), props.role))}
          </Text>
          <Show when={state.detail() === "detailed"}>
            <Text class={state.text().small} style={{ textColor: state.theme().dim }}>
              {`${game().id}  ·  ${state.categoryLabel(game().category)}`}
            </Text>
          </Show>
        </View>
      )}
    </Show>
  );
}
