//! The shelf's English texts (`src/locales/en.ts`). Phase 4 brings the other
//! four languages; until then the screens read their words from here.

pub const LAUNCH: &str = "Launch";
pub const EDIT: &str = "Edit";
pub const SEARCH: &str = "Search";
pub const CLOSE: &str = "Close";
pub const MENU: &str = "Menu";
pub const START: &str = "Start";
pub const BACK: &str = "Back";
pub const ADR_TITLE: &str = "Adrenaline plugin";
pub const ADR_ADD: &str = "Add plugin";
pub const ADR_TURN_ON: &str = "Turn on";
pub const ADR_FAILED: &str = "Adrenaline's plugin lists could not be changed.";

/// Why a title with no bubble cannot start yet, by the plugin's state.
pub fn adrenaline(state: crate::host::PluginState, title: &str) -> String {
    use crate::host::PluginState::*;
    match state {
        Off => format!("Pocket Shelf's plugin is turned off in Adrenaline's plugin lists. Without it, Adrenaline opens its menu instead of {title}. Turn it on and start the game?"),
        Missing => format!("{title} starts through Adrenaline. Pocket Shelf adds its own plugin to Adrenaline's plugin lists (ux0:pspemu/seplugins), which starts the game when Adrenaline opens. Add it and start the game?"),
        NoAdrenaline | On => format!("{title} starts through Adrenaline, and Adrenaline is not installed."),
    }
}
pub const EMPTY_CATEGORY: &str = "Nothing in this category yet";
pub const NO_TITLES: &str = "No installed apps found";
pub const MOVE_HERE: &str = "Press {button} on a title to move it here";

/// "Launching {title}...".
pub fn launching(title: &str) -> String {
    format!("Launching {title}...")
}
