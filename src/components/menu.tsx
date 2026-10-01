import { For } from "solid-js";
import { FONTS } from "../text.ts";
import { DETAIL_LABELS, VIEW_LABELS } from "../navigation.ts";
import type { LauncherState } from "../state.ts";
import { hints, Prompt, type IconName } from "./icons.tsx";
import { Drawer, OptionRow, Spacer } from "./panel.tsx";

interface Row {
  label: string;
  value?: string;
  icon?: IconName;
  step: "both" | "right";
}

/** The SELECT menu. */
export function MenuOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const rows = (): Row[] => [
    { label: "Theme", value: state.theme().name, step: "both" },
    { label: "Font", value: FONTS.find((item) => item.id === state.font())?.name ?? "", step: "both" },
    { label: "View", value: VIEW_LABELS[state.view()], step: "both" },
    { label: "Details", value: DETAIL_LABELS[state.detail()], step: "both" },
    { label: "Backdrop", value: state.backdropOn() ? "On" : "Off", step: "both" },
    { label: "Categories", value: "Manage", step: "right" },
    { label: "Confirm", icon: state.confirmButton(), step: "both" },
  ];
  return (
    <Drawer state={state} title="Menu">
      <For each={rows()}>
        {(row, index) => (
          <OptionRow
            state={state}
            active={state.menuRow() === index()}
            label={row.label}
            value={row.value}
            icon={row.icon}
            step={row.step}
          />
        )}
      </For>
      <Spacer />
      <Prompt
        state={state}
        parts={hints(["dpadVertical", "Move"], ["dpadHorizontal", "Change"], [state.cancelButton(), "Close"])}
      />
    </Drawer>
  );
}
