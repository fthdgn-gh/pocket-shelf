//! SELECT ▸ System ▸ Console start ([`crate::autostart`]): Open at console start,
//! Return after a game, and the plugin that does them. With AutoBoot in the
//! tai config, Open at console start sets the title AutoBoot starts (and
//! puts back the one it had); otherwise both need Pocket Shelf's plugin,
//! which the drawer offers to install. A change to the tai config, or the
//! plugin's removal, waits for a second press.

use super::chrome::Button;
use super::navigation::{Direction, Step as Nav};
use super::panel::{self, Drawer, Step};
use super::Shelf;
use crate::autostart::{Change, Setup, AUTOSTART_FILE, SELF_ID};
use crate::gfx::Gfx;
use crate::host::Host;
use crate::i18n::{fill, upper};

const BOOT_ROW: usize = 0;
const RETURN_ROW: usize = 1;
const PLUGIN_ROW: usize = 2;

/// What a second press does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Armed {
    /// Install Pocket Shelf's plugin, then turn this row's setting on.
    Install(usize),
    Remove,
}

pub struct ConsoleStart {
    row: usize,
    /// The host's last answer; None while it reads.
    setup: Option<Setup>,
    armed: Option<Armed>,
    /// After an install: the row whose setting goes on.
    then: Option<usize>,
    /// The change the host is doing.
    asked: Option<Change>,
    note: String,
}

impl ConsoleStart {
    pub(super) fn row(&self) -> usize {
        self.row
    }

    pub(super) fn set_row(&mut self, row: usize) {
        if row != self.row {
            self.row = row;
            self.armed = None;
            self.note.clear();
        }
    }
}

impl Shelf {
    pub(super) fn open_console_start(&mut self, host: &mut dyn Host) {
        let asked = host.autostart(Change::Check);
        self.console_start = Some(ConsoleStart { row: BOOT_ROW, setup: (!asked).then(Setup::default), armed: None, then: None, asked: None, note: String::new() });
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
    }

    fn save_autostart(&mut self, host: &mut dyn Host) {
        host.write(AUTOSTART_FILE, self.settings.autostart.render().as_bytes());
    }

    /// Ask the host for a change; the drawer waits for its answer.
    fn autostart_change(&mut self, host: &mut dyn Host, change: Change) {
        host.log(&format!("console start: {change:?}"));
        let asked = host.autostart(change.clone());
        if let Some(start) = &mut self.console_start {
            start.armed = None;
            if asked {
                start.setup = None;
                start.asked = Some(change);
            }
        }
    }

    /// The host's answer: what is installed, after a change.
    pub fn autostart_event(&mut self, host: &mut dyn Host, setup: Setup) {
        let t = self.t();
        host.log(&format!("console start: {setup:?}"));
        // With AutoBoot, it opens Pocket Shelf at console start; Pocket
        // Shelf's plugin must not too.
        if setup.found.autoboot && self.settings.autostart.boot {
            self.settings.autostart.boot = false;
            self.save_autostart(host);
        }
        let Some(start) = &mut self.console_start else { return };
        let then = start.then.take();
        if !setup.error.is_empty() {
            start.note = fill(t.start_failed, &[("error", &setup.error)]);
        } else if let Some(row) = then {
            if setup.found.ours {
                start.note = t.start_installed.to_string();
                match row {
                    BOOT_ROW if !setup.found.autoboot => self.settings.autostart.boot = true,
                    RETURN_ROW => self.settings.autostart.back = true,
                    _ => {}
                }
            }
        }
        let removed = start.asked.take() == Some(Change::Remove) && !setup.found.ours && setup.error.is_empty();
        if removed {
            start.note = t.start_removed.to_string();
            self.settings.autostart.boot = false;
            self.settings.autostart.back = false;
        }
        if let Some(start) = &mut self.console_start {
            start.setup = Some(setup);
        }
        if then.is_some() || removed {
            self.save_autostart(host);
        }
    }

    pub(super) fn console_start_input(&mut self, host: &mut dyn Host, step: Option<Nav>, confirm: bool, cancel: bool) {
        let t = self.t();
        let Some(start) = &mut self.console_start else { return };
        if cancel {
            self.console_start = None;
            return;
        }
        let toggle = match step {
            Some(Nav::Press(Direction::Up)) | Some(Nav::Repeat(Direction::Up, _)) => {
                start.row = (start.row + 2) % 3;
                start.armed = None;
                start.note.clear();
                return;
            }
            Some(Nav::Press(Direction::Down)) | Some(Nav::Repeat(Direction::Down, _)) => {
                start.row = (start.row + 1) % 3;
                start.armed = None;
                start.note.clear();
                return;
            }
            Some(Nav::Press(Direction::Left | Direction::Right)) => start.row != PLUGIN_ROW,
            _ => confirm,
        };
        if !toggle {
            return;
        }
        // Still reading: nothing to change yet.
        let Some(setup) = start.setup.clone() else { return };
        let row = start.row;
        start.note.clear();
        if setup.config.is_empty() {
            start.note = t.start_no_taihen.to_string();
            return;
        }
        // The second press of an armed change does it.
        match start.armed {
            Some(Armed::Install(armed)) if armed == row || row == PLUGIN_ROW => {
                start.then = Some(armed);
                self.autostart_change(host, Change::Install);
                return;
            }
            Some(Armed::Remove) if row == PLUGIN_ROW => {
                self.autostart_change(host, Change::Remove);
                return;
            }
            _ => start.armed = None,
        }
        match row {
            BOOT_ROW if setup.found.autoboot => {
                // AutoBoot starts Pocket Shelf, or the title it had before.
                let settings = &mut self.settings.autostart;
                let title = if setup.autoboot_title == SELF_ID {
                    std::mem::take(&mut settings.autoboot_before)
                } else {
                    settings.autoboot_before = setup.autoboot_title.clone();
                    SELF_ID.to_string()
                };
                self.save_autostart(host);
                self.autostart_change(host, Change::AutoBoot(title));
            }
            BOOT_ROW | RETURN_ROW if setup.found.ours => {
                let settings = &mut self.settings.autostart;
                if row == BOOT_ROW {
                    settings.boot = !settings.boot;
                } else {
                    settings.back = !settings.back;
                }
                self.save_autostart(host);
            }
            BOOT_ROW | RETURN_ROW => start.armed = Some(Armed::Install(row)),
            PLUGIN_ROW if setup.found.ours => start.armed = Some(Armed::Remove),
            _ => start.armed = Some(Armed::Install(PLUGIN_ROW)),
        }
    }

    pub(super) fn draw_console_start(&mut self, gfx: &mut Gfx) {
        let Some(start) = &self.console_start else { return };
        let t = self.t();
        let theme = self.theme;
        let slide = self.drawer.at(self.now);
        let mut drawer = Drawer::open(gfx, &theme, &upper(t.console_start, self.settings.language), panel::Size::Wide, slide);
        let on_off = |on: bool| if on { t.on } else { t.off };
        let settings = &self.settings.autostart;
        let (boot, back, plugin) = match &start.setup {
            None => ("-", "-", t.start_checking),
            Some(setup) => {
                let plugin = match (setup.found.autoboot, setup.found.ours) {
                    _ if setup.config.is_empty() => t.start_none,
                    (true, true) => t.start_both,
                    (true, false) => "AutoBoot",
                    (false, true) => t.start_ours,
                    (false, false) => t.start_none,
                };
                (on_off(setup.opens_at_start(settings)), on_off(setup.found.ours && settings.back), plugin)
            }
        };
        drawer.row(gfx, t.open_at_start, Some(boot), None, start.row == BOOT_ROW, Step::Both);
        drawer.row(gfx, t.return_after_game, Some(back), None, start.row == RETURN_ROW, Step::Both);
        drawer.row(gfx, t.start_plugin, Some(plugin), None, start.row == PLUGIN_ROW, Step::Right);
        let config = start.setup.as_ref().map_or("", |s| s.config.as_str());
        let note = match start.armed {
            Some(Armed::Install(_)) => fill(t.start_install_confirm, &[("config", config)]),
            Some(Armed::Remove) => fill(t.start_remove_confirm, &[("config", config)]),
            None if !start.note.is_empty() => start.note.clone(),
            None => match &start.setup {
                None => t.start_checking.to_string(),
                Some(setup) if setup.config.is_empty() => t.start_no_taihen.to_string(),
                Some(setup) if setup.found.autoboot => t.start_help_autoboot.to_string(),
                Some(_) => t.start_help.to_string(),
            },
        };
        drawer.note(gfx, &note, 1);
        let confirm = Button::confirm(self.settings.confirm);
        let action = match (start.armed, start.row, start.setup.as_ref().is_some_and(|s| s.found.ours)) {
            (Some(Armed::Install(_)), _, _) | (None, PLUGIN_ROW, false) => t.start_install,
            (Some(Armed::Remove), _, _) | (None, PLUGIN_ROW, true) => t.remove,
            _ => t.change,
        };
        drawer.hints(gfx, &[(confirm, action), (Button::cancel(self.settings.confirm), t.back)]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autostart::{AutoStart, Found, RETURN_FILE};
    use crate::host::{Event, PictureKey, PluginState, Status, TitleInfo};
    use crate::library::catalog::load_catalog;
    use crate::library::settings::{Settings, UpdateChannel};
    use crate::library::testing::Files;
    use crate::library::Library;

    /// A data folder that records the changes asked for.
    #[derive(Default)]
    struct StartHost {
        files: Files,
        changes: Vec<Change>,
    }

    impl Host for StartHost {
        fn time_us(&self) -> u64 {
            0
        }
        fn read(&mut self, path: &str) -> Option<Vec<u8>> {
            self.files.read(path)
        }
        fn write(&mut self, path: &str, data: &[u8]) -> bool {
            self.files.write(path, data)
        }
        fn remove(&mut self, path: &str) -> bool {
            self.files.remove(path)
        }
        fn list(&mut self, _: &str) -> Vec<String> {
            Vec::new()
        }
        fn status(&mut self) -> Status {
            Status::default()
        }
        fn scan(&mut self, _: bool) {}
        fn request(&mut self, _: PictureKey) {}
        fn poll(&mut self, _: &mut Vec<Event>) {}
        fn log(&mut self, _: &str) {}
        fn launch(&mut self, _: &str, _: Option<crate::host::Emulator>) -> bool {
            true
        }
        fn plugin_state(&mut self) -> PluginState {
            PluginState::On
        }
        fn enable_plugin(&mut self) -> bool {
            true
        }
        fn autostart(&mut self, change: Change) -> bool {
            self.changes.push(change);
            true
        }
    }

    const CONFIG: &str = "ux0:tai/config.txt";

    fn setup(ours: bool, autoboot: Option<&str>) -> Setup {
        Setup { config: CONFIG.into(), found: Found { ours, autoboot: autoboot.is_some() }, autoboot_title: autoboot.unwrap_or("").into(), error: String::new() }
    }

    fn shelf(host: &mut StartHost) -> Shelf {
        let library = Library::load(host);
        let mut shelf = Shelf::new(Settings::defaults(UpdateChannel::Off), library, Status::default());
        let infos = [TitleInfo { id: "PCSA00069".into(), title: "Gravity".into(), platform: "vita".into(), ..Default::default() }];
        shelf.set_titles(load_catalog(&infos), true);
        shelf
    }

    fn saved(host: &mut StartHost) -> AutoStart {
        host.read(crate::autostart::AUTOSTART_FILE).map(|b| AutoStart::parse(&b)).unwrap_or_default()
    }

    #[test]
    fn our_plugin_is_installed_on_a_second_press_and_its_settings_saved() {
        let mut host = StartHost::default();
        let mut shelf = shelf(&mut host);
        shelf.open_console_start(&mut host);
        assert_eq!(host.changes, [Change::Check]);
        shelf.autostart_event(&mut host, setup(false, None));

        // Open at console start: the first press asks, the second installs.
        shelf.console_start_input(&mut host, None, true, false);
        assert_eq!(host.changes.len(), 1);
        shelf.console_start_input(&mut host, None, true, false);
        assert_eq!(host.changes[1], Change::Install);
        shelf.autostart_event(&mut host, setup(true, None));
        assert_eq!(saved(&mut host), AutoStart { boot: true, ..Default::default() });

        // Return after a game: on at once now; a launch leaves the marker.
        shelf.console_start_input(&mut host, Some(Nav::Press(Direction::Down)), false, false);
        shelf.console_start_input(&mut host, None, true, false);
        assert_eq!(saved(&mut host), AutoStart { boot: true, back: true, ..Default::default() });
        shelf.console_start_input(&mut host, None, false, true);
        assert!(shelf.launch_selected(&mut host).is_some());
        assert_eq!(host.read(RETURN_FILE).as_deref(), Some(&b"PCSA00069"[..]));

        // The plugin row removes it on a second press; both go off.
        shelf.open_console_start(&mut host);
        shelf.autostart_event(&mut host, setup(true, None));
        shelf.console_start_input(&mut host, Some(Nav::Press(Direction::Up)), false, false);
        shelf.console_start_input(&mut host, None, true, false);
        assert_eq!(host.changes.last(), Some(&Change::Check));
        shelf.console_start_input(&mut host, None, true, false);
        assert_eq!(host.changes.last(), Some(&Change::Remove));
        shelf.autostart_event(&mut host, setup(false, None));
        assert_eq!(saved(&mut host), AutoStart::default());
    }

    #[test]
    fn with_autoboot_console_start_sets_its_title_and_puts_the_old_one_back() {
        let mut host = StartHost::default();
        let mut shelf = shelf(&mut host);
        // Pocket Shelf's own boot goes off when AutoBoot is there.
        shelf.settings.autostart.boot = true;
        shelf.open_console_start(&mut host);
        shelf.autostart_event(&mut host, setup(true, Some("PCSB00245")));
        assert!(!saved(&mut host).boot);

        shelf.console_start_input(&mut host, None, true, false);
        assert_eq!(host.changes.last(), Some(&Change::AutoBoot("POCKTSHLF".into())));
        assert_eq!(saved(&mut host).autoboot_before, "PCSB00245");
        shelf.autostart_event(&mut host, setup(true, Some("POCKTSHLF")));
        shelf.console_start_input(&mut host, None, true, false);
        assert_eq!(host.changes.last(), Some(&Change::AutoBoot("PCSB00245".into())));
        assert_eq!(saved(&mut host).autoboot_before, "");

        // No taiHEN: nothing to change.
        shelf.autostart_event(&mut host, Setup::default());
        let asked = host.changes.len();
        shelf.console_start_input(&mut host, None, true, false);
        shelf.console_start_input(&mut host, None, true, false);
        assert_eq!(host.changes.len(), asked);
    }
}
