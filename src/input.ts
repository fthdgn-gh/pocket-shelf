import { onCleanup } from "solid-js";
import type { NodeMirror } from "@pocketjs/framework/components";
import { BTN, pushFocusController } from "@pocketjs/framework/input";
import { onButtonPress } from "@pocketjs/framework/lifecycle";
import { moveSelection } from "./navigation.ts";
import type { LauncherState } from "./state.ts";

/**
 * Bind every launcher button. Call once from `onMount`, with the root node
 * that contains the views.
 *
 *  - D-pad: moves the selection in the current view, or drives the panel that
 *    is open (SELECT menu, title editor, keyboard, art picker).
 *  - L / R: previous / next category (shift / symbols on the keyboard).
 *  - Confirm button (circle or cross): launch, or act in the open panel.
 *  - The other face button: back out of the open panel.
 *  - Triangle: edit the selected title. SELECT: menu.
 *  - Square and START: delete a character and finish, on the keyboard.
 *  - Category manager: triangle new, square delete, START hide / show, L / R move up / down.
 */
export function installInput(state: LauncherState, root: NodeMirror): void {
  // The controller claims every D-pad press so the framework's own focus
  // traversal never moves focus away from the selected item.
  onCleanup(
    pushFocusController(root, (direction) => {
      if (state.modal()) return true;
      const next = moveSelection(state.view(), direction, state.selectedIndex(), state.games().length);
      if (next !== null) state.select(next);
      return true;
    }),
  );

  const closed = () => state.modal() === null;
  const typing = () => state.modal() === "keyboard";
  onButtonPress(BTN.LTRIGGER, () => state.changeCategory(-1), { active: closed });
  onButtonPress(BTN.RTRIGGER, () => state.changeCategory(1), { active: closed });
  onButtonPress(BTN.LTRIGGER, state.toggleShift, { active: typing });
  onButtonPress(BTN.RTRIGGER, state.toggleSymbols, { active: typing });

  const confirm = () => {
    if (state.modal()) state.modalConfirm();
    else state.launchSelected();
  };
  const cancel = () => state.modalCancel();
  onButtonPress(BTN.CIRCLE, () => (state.confirmMode() === "circle" ? confirm() : cancel()));
  onButtonPress(BTN.CROSS, () => (state.confirmMode() === "cross" ? confirm() : cancel()));
  onButtonPress(BTN.SELECT, () => {
    const modal = state.modal();
    if (modal === null || modal === "menu") state.toggleMenu();
  });
  onButtonPress(BTN.TRIANGLE, state.openEditor, { active: closed });
  onButtonPress(BTN.SQUARE, state.backspace, { active: typing });
  onButtonPress(BTN.START, state.commitKeyboard, { active: typing });
  const managing = () => state.modal() === "categories";
  onButtonPress(BTN.TRIANGLE, state.catNew, { active: managing });
  onButtonPress(BTN.SQUARE, state.catDelete, { active: managing });
  onButtonPress(BTN.START, state.catToggleHidden, { active: managing });
  onButtonPress(BTN.LTRIGGER, () => state.catReorder(-1), { active: managing });
  onButtonPress(BTN.RTRIGGER, () => state.catReorder(1), { active: managing });

  // D-pad while a panel is open. The controller above swallows the same
  // presses for the views, so these are the only consumers.
  const open = () => state.modal() !== null;
  onButtonPress(BTN.UP, () => state.modalMove(0, -1), { active: open });
  onButtonPress(BTN.DOWN, () => state.modalMove(0, 1), { active: open });
  onButtonPress(BTN.LEFT, () => state.modalMove(-1, 0), { active: open });
  onButtonPress(BTN.RIGHT, () => state.modalMove(1, 0), { active: open });
}
