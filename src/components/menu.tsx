import { For } from "solid-js";
import { LANGUAGES } from "../i18n.ts";
import { FONTS } from "../text.ts";
import { BUILD, buildLabel } from "../updates.ts";
import { MENU_VISIBLE, type LauncherState, type MenuItem } from "../state.ts";
import { hints, Prompt, type IconName } from "./icons.tsx";
import { Drawer, OptionRow, Spacer } from "./panel.tsx";

interface Row {
  label: string;
  value?: string;
  icon?: IconName;
  step: "both" | "right";
}

/** The SELECT menu: the main page, or the group opened from it. */
export function MenuOverlay(props: { state: LauncherState }) {
  const { state } = props;
  const t = state.t;
  const onOff = (on: boolean) => (on ? t().on : t().off);
  const row = (item: MenuItem): Row => {
    switch (item) {
      case "main":
        return { label: t().menu, step: "right" };
      case "appearance":
        return { label: t().appearance, step: "right" };
      case "status":
        return { label: t().statusBar, step: "right" };
      case "library":
        return { label: t().library, step: "right" };
      case "updates":
        return { label: t().updates, step: "right" };
      case "theme":
        return { label: t().theme, value: t().themes[state.theme().id], step: "both" };
      case "font":
        return { label: t().font, value: FONTS.find((entry) => entry.id === state.font())?.name ?? "", step: "both" };
      case "view":
        return { label: t().view, value: t().views[state.view()], step: "both" };
      case "details":
        return { label: t().details, value: t().detailLevels[state.detail()], step: "both" };
      case "backdrop":
        return { label: t().backdrop, value: onOff(state.backdropOn()), step: "both" };
      case "iconBox":
        return { label: t().iconBox, value: onOff(state.iconBoxOn()), step: "both" };
      case "statusBar":
        return { label: t().showStatusBar, value: onOff(state.statusBarOn()), step: "both" };
      case "clock":
        return { label: t().clock, value: t().clockFormats[state.clockFormat()], step: "both" };
      case "batteryPercent":
        return { label: t().batteryPercent, value: onOff(state.batteryPercentOn()), step: "both" };
      case "categories":
        return { label: t().categoriesRow, value: t().manage, step: "right" };
      case "fetchArt":
        return { label: t().fetchArt, value: "SteamGridDB", step: "right" };
      case "cleanArt":
        return { label: t().cleanArt, step: "right" };
      case "rescan":
        return { label: t().rescan, step: "right" };
      case "updateChannel":
        return { label: t().updateChannel, value: t().updateChannels[state.updateChannel()], step: "both" };
      case "checkUpdates":
        return { label: t().checkUpdates, value: buildLabel(BUILD), step: "right" };
      case "confirm":
        return { label: t().confirm, icon: state.confirmButton(), step: "both" };
      case "language":
        // Each language under its own name, whatever the current one is.
        return {
          label: t().language,
          value: LANGUAGES.find((entry) => entry.id === state.language())?.name ?? "",
          step: "both",
        };
      case "diagnostics":
        return { label: t().diagnostics, step: "right" };
    }
  };
  const rows = (): Row[] => state.menuItems().map(row);
  const main = () => state.menuPage() === "main";
  // The menu has more rows than fit: it scrolls so the highlighted row stays in view.
  const first = () => {
    const last = rows().length - MENU_VISIBLE;
    return Math.max(0, Math.min(state.menuRow() - Math.floor(MENU_VISIBLE / 2), last));
  };
  const visible = () => rows().slice(first(), first() + MENU_VISIBLE);
  return (
    <Drawer state={state} title={row(state.menuPage()).label} size="wide">
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
          [state.cancelButton(), main() ? t().close : t().back],
        )}
      />
    </Drawer>
  );
}
