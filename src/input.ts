import { BTN, type FocusDirection } from "@pocketjs/framework/input";
import { onButtonPress, onFrame } from "@pocketjs/framework/lifecycle";
import { moveSelection } from "./navigation.ts";
import type { LauncherState } from "./state.ts";

// A held direction moves once, waits, then repeats. Counted in host frames
// (60 per second on the Vita): 0.4 s before the first repeat, then 10 per second.
const REPEAT_DELAY = 24;
const REPEAT_EVERY = 6;

const DIRECTIONS: readonly [mask: number, direction: FocusDirection, dx: number, dy: number][] = [
  [BTN.LEFT, "left", -1, 0],
  [BTN.RIGHT, "right", 1, 0],
  [BTN.UP, "up", 0, -1],
  [BTN.DOWN, "down", 0, 1],
];

/**
 * Bind every launcher button. Call once from `onMount`.
 *
 *  - D-pad: moves the selection in the current view, or drives the panel that
 *    is open (SELECT menu, title editor, keyboard, art picker). Holding a
 *    direction repeats it.
 *  - L / R: previous / next category (shift / next page on the keyboard).
 *  - Confirm button (circle or cross): launch, or act in the open panel.
 *  - The other face button: back out of the open panel.
 *  - Triangle: edit the selected title. Square: search. SELECT: menu.
 *  - On the search tab, the back button closes the search.
 *  - Square and START: delete a character and finish, on the keyboard.
 *  - Category manager: triangle new, square delete, START hide / show, L / R move up / down.
 *
 * Nothing on screen is focusable: the launcher owns the selection, so the
 * framework's focus traversal and its press of the focused node stay idle.
 */
export function installInput(state: LauncherState): void {
  const move = (direction: FocusDirection, dx: number, dy: number, repeated: boolean) => {
    const modal = state.modal();
    if (modal === null) {
      const next = moveSelection(state.view(), direction, state.selectedIndex(), state.games().length);
      if (next !== null) state.select(next);
      return;
    }
    // A repeat scrolls rows and keys. It does not step a row's value, which
    // would run through themes or open another panel while the button is held.
    if (repeated && dy === 0 && modal !== "keyboard") return;
    state.modalMove(dx, dy);
  };

  let held = 0;
  let heldFrames = 0;
  onFrame((buttons) => {
    state.frame();
    const entry = DIRECTIONS.find(([mask]) => buttons & mask);
    if (!entry) {
      held = 0;
      return;
    }
    const [mask, direction, dx, dy] = entry;
    if (mask !== held) {
      held = mask;
      heldFrames = 0;
      move(direction, dx, dy, false);
      return;
    }
    heldFrames++;
    if (heldFrames >= REPEAT_DELAY && (heldFrames - REPEAT_DELAY) % REPEAT_EVERY === 0) {
      move(direction, dx, dy, true);
    }
  });

  const closed = () => state.modal() === null;
  const typing = () => state.modal() === "keyboard";
  onButtonPress(BTN.LTRIGGER, () => state.changeCategory(-1), { active: closed });
  onButtonPress(BTN.RTRIGGER, () => state.changeCategory(1), { active: closed });
  onButtonPress(BTN.LTRIGGER, state.toggleShift, { active: typing });
  onButtonPress(BTN.RTRIGGER, state.nextKeyPage, { active: typing });

  const confirm = () => {
    if (state.modal()) state.modalConfirm();
    else state.launchSelected();
  };
  const cancel = () => (state.modal() ? state.modalCancel() : state.closeSearch());
  onButtonPress(BTN.CIRCLE, () => (state.confirmMode() === "circle" ? confirm() : cancel()));
  onButtonPress(BTN.CROSS, () => (state.confirmMode() === "cross" ? confirm() : cancel()));
  onButtonPress(BTN.SELECT, () => {
    const modal = state.modal();
    if (modal === null || modal === "menu") state.toggleMenu();
  });
  onButtonPress(BTN.TRIANGLE, state.openEditor, { active: closed });
  // One handler each for square and START. With one handler per panel, a
  // press that opens or closes a panel also reached the handler of the panel
  // it led to: opening the search keyboard removed a character.
  onButtonPress(BTN.SQUARE, () => {
    const modal = state.modal();
    if (modal === null) state.openSearch();
    else if (modal === "keyboard") state.backspace();
    else if (modal === "categories") state.catDelete();
  });
  onButtonPress(BTN.START, () => {
    const modal = state.modal();
    if (modal === "keyboard") state.commitKeyboard();
    else if (modal === "categories") state.catToggleHidden();
  });
  const managing = () => state.modal() === "categories";
  onButtonPress(BTN.TRIANGLE, state.catNew, { active: managing });
  onButtonPress(BTN.LTRIGGER, () => state.catReorder(-1), { active: managing });
  onButtonPress(BTN.RTRIGGER, () => state.catReorder(1), { active: managing });
}
