import { For } from "solid-js";
import { FONTS } from "../text.ts";
import { DETAIL_LABELS, VIEW_LABELS } from "../navigation.ts";
import type { LauncherState } from "../state.ts";
import { Drawer, Note, OptionRow, Spacer } from "./panel.tsx";

/** The SELECT menu. */
export function MenuOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const rows = () => [
    { label: "Theme", value: state.theme().name },
    { label: "Font", value: FONTS.find((item) => item.id === state.font())?.name ?? "" },
    { label: "View", value: VIEW_LABELS[state.view()] },
    { label: "Details", value: DETAIL_LABELS[state.detail()] },
    { label: "Categories", value: "Manage" },
    { label: "Confirm", value: state.confirmGlyph() },
  ];
  return (
    <Drawer state={state} title="Menu">
      <For each={rows()}>
        {(row, index) => {
          const active = () => state.menuRow() === index();
          return (
            <OptionRow
              state={state}
              active={active()}
              label={row.label}
              value={active() ? `◄ ${row.value} ►` : row.value}
            />
          );
        }}
      </For>
      <Spacer />
      <Note state={state}>{`D-pad: Move / Change  ·  ${state.cancelGlyph()}: Close`}</Note>
    </Drawer>
  );
}
