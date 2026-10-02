import { For } from "solid-js";
import { LANGUAGES } from "../i18n.ts";
import { FONTS } from "../text.ts";
import { MENU_VISIBLE, type LauncherState } from "../state.ts";
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
  const t = state.t;
  const onOff = (on: boolean) => (on ? t().on : t().off);
  const rows = (): Row[] => [
    { label: t().theme, value: t().themes[state.theme().id], step: "both" },
    { label: t().font, value: FONTS.find((item) => item.id === state.font())?.name ?? "", step: "both" },
    { label: t().view, value: t().views[state.view()], step: "both" },
    { label: t().details, value: t().detailLevels[state.detail()], step: "both" },
    { label: t().backdrop, value: onOff(state.backdropOn()), step: "both" },
    { label: t().iconBox, value: onOff(state.iconBoxOn()), step: "both" },
    { label: t().categoriesRow, value: t().manage, step: "right" },
    { label: t().fetchArt, value: "SteamGridDB", step: "right" },
    { label: t().cleanArt, step: "right" },
    { label: t().rescan, step: "right" },
    { label: t().confirm, icon: state.confirmButton(), step: "both" },
    // Each language under its own name, whatever the current one is.
    { label: t().language, value: LANGUAGES.find((item) => item.id === state.language())?.name ?? "", step: "both" },
    { label: t().diagnostics, step: "right" },
  ];
  // The menu has more rows than fit: it scrolls so the highlighted row stays in view.
  const first = () => {
    const last = rows().length - MENU_VISIBLE;
    return Math.max(0, Math.min(state.menuRow() - Math.floor(MENU_VISIBLE / 2), last));
  };
  const visible = () => rows().slice(first(), first() + MENU_VISIBLE);
  return (
    <Drawer state={state} title={t().menu} size="wide">
      <For each={visible()}>
        {(row, index) => (
          <OptionRow
            state={state}
            active={state.menuRow() === first() + index()}
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
        parts={hints(
          ["dpadVertical", t().navigate],
          ["dpadHorizontal", t().change],
          [state.cancelButton(), t().close],
        )}
      />
    </Drawer>
  );
}
