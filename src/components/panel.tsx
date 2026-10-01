import { onMount, Show, type JSX } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import type { LauncherState } from "../state.ts";
import { alpha } from "../themes.ts";
import { Icon, type IconName } from "./icons.tsx";

const DRAWER = {
  narrow: { width: 232, panel: "absolute top-0 right-0 bottom-0 w-[232] flex-col gap-[3] pl-4 pr-3 py-3" },
  wide: { width: 284, panel: "absolute top-0 right-0 bottom-0 w-[284] flex-col gap-[3] pl-4 pr-3 py-3" },
} as const;

/**
 * A panel that slides in from the right edge over a dimmed screen. The SELECT
 * menu, the title editor, the art picker and the category manager use it.
 */
export function Drawer(props: {
  state: LauncherState;
  title: string;
  size?: keyof typeof DRAWER;
  children: JSX.Element;
}) {
  const shape = DRAWER[props.size ?? "narrow"];
  let panelRef: NodeMirror | undefined;
  onMount(() => {
    if (panelRef) animate(panelRef, "translateX", 0, { dur: 180, easing: "out" });
  });
  return (
    <View class="absolute inset-0" style={{ bgColor: props.state.theme().scrim }}>
      <View
        ref={(el) => {
          panelRef = el;
        }}
        class={shape.panel}
        style={{ bgColor: props.state.theme().panel, translateX: shape.width }}
      >
        <View class="absolute top-0 left-0 bottom-0 w-[2]" style={{ bgColor: props.state.theme().accent }} />
        <Text class={props.state.text().label} style={{ textColor: props.state.theme().accent }}>
          {props.title.toUpperCase()}
        </Text>
        <View class="h-[4] shrink-0" />
        {props.children}
      </View>
    </View>
  );
}

/**
 * One line of a drawer: a label on the left and its value on the right. The
 * value is text, an icon, or both. `step` adds the arrows that show the value
 * changes with left and right: "both" for a list of choices, "right" for a
 * row that opens something.
 */
export function OptionRow(props: {
  state: LauncherState;
  active: boolean;
  label: string;
  value?: string;
  icon?: IconName;
  step?: "both" | "right";
}) {
  const theme = props.state.theme;
  const arrows = () => (props.active ? props.step : undefined);
  return (
    <View
      class="flex-row items-center justify-between w-full h-[26] shrink-0 px-2 rounded-md"
      style={{ bgColor: props.active ? alpha(theme().accent, "30") : "#00000000" }}
    >
      <Text
        class={props.active ? props.state.text().bodyBold : props.state.text().body}
        style={{ textColor: props.active ? theme().text : theme().dim }}
      >
        {props.label}
      </Text>
      <View class="flex-row items-center gap-1 shrink-0">
        <Show when={arrows() === "both"}>
          <Icon name="arrowLeft" />
        </Show>
        <Show when={props.value}>
          <Text class={props.state.text().bodyBold} style={{ textColor: props.active ? theme().accent : theme().dim }}>
            {props.value}
          </Text>
        </Show>
        <Show when={props.icon}>{(name) => <Icon name={name()} />}</Show>
        <Show when={arrows()}>
          <Icon name="arrowRight" />
        </Show>
      </View>
    </View>
  );
}

/** Pushes whatever follows it to the bottom of a drawer. */
export function Spacer() {
  return <View class="grow" />;
}

/** A line of dim text at the bottom of a panel. */
export function Note(props: { state: LauncherState; children: JSX.Element }) {
  return (
    <Text class={props.state.text().small} style={{ textColor: props.state.theme().dim }}>
      {props.children}
    </Text>
  );
}
