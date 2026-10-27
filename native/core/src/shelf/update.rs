//! Checking for a newer build of the chosen channel and installing it
//! (`src/update-flow.ts`, `components/update.tsx`; SELECT > Check for
//! updates).

use super::chrome::Button;
use super::panel::{self, Drawer, Step};
use super::Shelf;
use crate::gfx::{Gfx, Rect};
use crate::host::{Host, NetId, NetKind, NetState};
use crate::i18n::{fill, upper};
use crate::library::settings::UpdateChannel;
use crate::updates::{self, build_label, check_due, parse_releases, pick_update, release_key, release_label, Release, UpdateRecord};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum UpdateStep {
    /// Asking GitHub for its releases.
    #[default]
    Checking,
    /// Nothing newer on the channel.
    Current,
    /// A release to download, with Download and Later.
    Available,
    Downloading,
    Unpacking,
    /// Unpacked and checked, with Install.
    Ready,
    /// Installing the updater app, which then takes over.
    Installing,
    /// The updater installed the release (shown after the restart).
    Updated,
    Failed,
}

/// The request in flight.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Net {
    /// The check: from the menu, or the silent one at start.
    Check { at_start: bool },
    Download,
}

/// What to show once no other drawer is open.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Pending {
    Offer(Release),
    Report(String),
}

#[derive(Default)]
pub struct UpdateFlow {
    pub open: bool,
    pub step: UpdateStep,
    pub status: String,
    pub release: Option<Release>,
    pub progress: (u64, u64),
    net: Option<(NetId, Net)>,
    /// A host step (unpack, install) runs: its state is read each frame.
    polling: bool,
    pending: Option<Pending>,
    /// The updater is to start once this frame is shown.
    pub launch: bool,
}

fn megabytes(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1024.0 * 1024.0))
}

impl Shelf {
    fn fail_update(&mut self, message: String) {
        self.update.step = UpdateStep::Failed;
        self.update.status = message;
    }

    fn stop_update_request(&mut self, host: &mut dyn Host) {
        if let Some((id, _)) = self.update.net.take() {
            host.net_close(id);
        }
    }

    fn ask_github(&mut self, host: &mut dyn Host, at_start: bool) -> bool {
        match host.net_start(&updates::releases_url(), NetKind::Text { authorization: String::new() }) {
            Some(id) => {
                self.update.net = Some((id, Net::Check { at_start }));
                true
            }
            None => false,
        }
    }

    /// SELECT > Check for updates: open the drawer and check now.
    pub(super) fn check_updates(&mut self, host: &mut dyn Host) {
        self.update.open = true;
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
        if self.update.polling {
            return;
        }
        self.stop_update_request(host);
        self.update.release = None;
        self.update.status.clear();
        self.update.step = UpdateStep::Checking;
        let t = self.t();
        if self.settings.updates == UpdateChannel::Off {
            self.fail_update(t.update_off.into());
        } else if !self.ask_github(host, false) {
            self.fail_update(t.no_network.into());
        }
    }

    /// At start: the updater's report when it ran, else the check, at most
    /// once a day and silent unless it finds a release not put off before.
    /// A build that cannot install (the side-by-side title id) does not
    /// check at start.
    pub(super) fn updates_at_start(&mut self, host: &mut dyn Host) {
        let report = host.update_result();
        if !report.is_empty() {
            self.update.pending = Some(Pending::Report(report));
            return;
        }
        let record = UpdateRecord::load(host);
        if host.can_install_update() && check_due(self.settings.updates, &record, host.unix_ms()) {
            self.ask_github(host, true);
        }
    }

    fn show_update(&mut self, release: Option<Release>) {
        self.update.step = if release.is_some() { UpdateStep::Available } else { UpdateStep::Current };
        self.update.release = release;
        self.update.status.clear();
    }

    /// Whether no drawer, menu or keyboard is open (something found at start
    /// waits for that).
    fn update_idle(&self) -> bool {
        !self.menu.open
            && self.keyboard.is_none()
            && self.editor.is_none()
            && self.picker.is_none()
            && self.online.is_none()
            && self.categories.is_none()
            && self.rescan.is_none()
            && self.diagnostics.is_none()
            && self.fetch.is_none()
            && self.cleanup.is_none()
            && self.adrenaline.is_none()
            && !self.update.open
    }

    pub(super) fn update_input(&mut self, host: &mut dyn Host, confirm: bool, cancel: bool) {
        let t = self.t();
        if confirm {
            match self.update.step {
                UpdateStep::Available => {
                    let Some(release) = self.update.release.clone() else { return };
                    if !host.can_install_update() {
                        self.fail_update(t.update_not_supported.into());
                        return;
                    }
                    match host.net_start(&release.url, NetKind::Save(updates::UPDATE_FILE.into())) {
                        Some(id) => {
                            self.update.net = Some((id, Net::Download));
                            self.update.step = UpdateStep::Downloading;
                            self.update.progress = (0, release.size);
                            self.update.status.clear();
                        }
                        None => self.fail_update(t.no_network.into()),
                    }
                }
                UpdateStep::Ready => {
                    // For the updater's screen; it leaves the version out
                    // without the file.
                    if let Some(release) = &self.update.release {
                        if !host.write(updates::UPDATE_VERSION_FILE, release_label(release).as_bytes()) {
                            host.log("Update version not saved");
                        }
                    }
                    self.run_update_step(host, UpdateStep::Installing);
                }
                UpdateStep::Failed | UpdateStep::Current => self.check_updates(host),
                UpdateStep::Updated => self.update.open = false,
                _ => {}
            }
        } else if cancel {
            // An offered release is put off; a download stops. A host step
            // runs on; the drawer only closes.
            if self.update.step == UpdateStep::Available {
                if let Some(release) = &self.update.release {
                    let mut record = UpdateRecord::load(host);
                    record.later = Some(release_key(release));
                    record.save(host);
                }
            }
            if self.update.step == UpdateStep::Installing {
                return;
            }
            if !self.update.polling {
                self.stop_update_request(host);
            }
            self.update.open = false;
        }
    }

    /// Start a host step and follow its state each frame.
    fn run_update_step(&mut self, host: &mut dyn Host, next: UpdateStep) {
        let started = match next {
            UpdateStep::Unpacking => host.update_unpack(),
            UpdateStep::Installing => host.update_install(),
            _ => false,
        };
        if !started {
            let t = self.t();
            self.fail_update(fill(t.update_failed, &[("error", "busy")]));
            return;
        }
        self.update.polling = true;
        self.update.step = next;
        self.update.progress = (0, 0);
        self.update.status.clear();
    }

    /// Once per frame: what the check at start found, the request in
    /// flight, and a running host step.
    pub(super) fn update_frame(&mut self, host: &mut dyn Host) {
        let t = self.t();
        if self.update.pending.is_some() && self.update_idle() {
            let pending = self.update.pending.take();
            self.update.open = true;
            self.drawer = panel::slide();
            self.drawer.set(1.0, self.now);
            match pending {
                Some(Pending::Offer(release)) => self.show_update(Some(release)),
                Some(Pending::Report(report)) => {
                    self.update.release = None;
                    let (kind, rest) = report.split_once(' ').unwrap_or((report.as_str(), ""));
                    if kind == "ok" {
                        self.update.step = UpdateStep::Updated;
                        // The running build is the one the updater installed.
                        self.update.status = fill(t.update_done, &[("version", &build_label(&self.build))]);
                    } else {
                        self.fail_update(fill(t.update_install_failed, &[("error", if rest.is_empty() { "unknown" } else { rest })]));
                    }
                }
                None => {}
            }
        }
        if let Some((id, net)) = self.update.net {
            match host.net_state(id) {
                NetState::Busy { received, total } => {
                    if net == Net::Download {
                        let size = self.update.release.as_ref().map_or(0, |r| r.size);
                        self.update.progress = (received, if total > 0 { total } else { size });
                    }
                }
                state => {
                    let text = if matches!(net, Net::Check { .. }) && matches!(state, NetState::Done { .. }) { host.net_text(id) } else { String::new() };
                    host.net_close(id);
                    self.update.net = None;
                    self.update_answer(host, net, state, text);
                }
            }
        }
        if !self.update.polling {
            return;
        }
        let state = host.update_state();
        let (kind, rest) = state.split_once(' ').unwrap_or((state.as_str(), ""));
        match kind {
            "busy" => {
                let mut numbers = rest.split(' ').map(|n| n.parse().unwrap_or(0));
                self.update.progress = (numbers.next().unwrap_or(0), numbers.next().unwrap_or(0));
            }
            "idle" | "installing" => {}
            "done" => {
                self.update.polling = false;
                self.update.step = UpdateStep::Ready;
                self.update.status = t.update_ready.into();
            }
            "installed" => {
                self.update.polling = false;
                self.update.open = true;
                self.update.status = t.update_restarting.into();
                // The process ends after this frame; the updater takes over.
                if host.update_launch() {
                    self.update.launch = true;
                } else {
                    self.fail_update(fill(t.update_install_failed, &[("error", "launch")]));
                }
            }
            _ => {
                self.update.polling = false;
                self.fail_update(fill(t.update_failed, &[("error", if rest.is_empty() { "unknown" } else { rest })]));
            }
        }
    }

    fn update_answer(&mut self, host: &mut dyn Host, net: Net, state: NetState, text: String) {
        let t = self.t();
        let status = match state {
            NetState::Done { status, .. } => status,
            _ => 0,
        };
        let error = match &state {
            NetState::Error(e) => Some(e.clone()),
            _ => None,
        };
        match net {
            Net::Check { at_start } => {
                let releases = if error.is_none() && status == 200 { parse_releases(&text) } else { None };
                let Some(releases) = releases else {
                    // The check at start is silent.
                    if !at_start {
                        let message = match error {
                            Some(e) => fill(t.update_unreachable, &[("error", &e)]),
                            None => fill(t.update_bad_reply, &[("status", &status.to_string())]),
                        };
                        self.fail_update(message);
                    }
                    return;
                };
                let mut record = UpdateRecord::load(host);
                record.checked_at = host.unix_ms();
                record.save(host);
                let found = pick_update(&releases, self.settings.updates, &self.build).cloned();
                if at_start {
                    if let Some(release) = found.filter(|r| Some(release_key(r)) != record.later) {
                        self.update.pending = Some(Pending::Offer(release));
                    }
                } else {
                    self.show_update(found);
                }
            }
            Net::Download => match error {
                Some(e) => self.fail_update(fill(t.download_failed, &[("error", &e)])),
                None if status != 200 => self.fail_update(fill(t.update_bad_reply, &[("status", &status.to_string())])),
                None => self.run_update_step(host, UpdateStep::Unpacking),
            },
        }
    }

    pub(super) fn draw_update(&mut self, gfx: &mut Gfx) {
        let t = self.t();
        let theme = self.theme;
        let slide = self.drawer.at(self.now);
        let flow = &self.update;
        let mut drawer = Drawer::open(gfx, &theme, &upper(t.updates, self.settings.language), panel::Size::Wide, slide);
        drawer.row(gfx, t.installed_version, Some(&build_label(&self.build)), None, false, Step::None);
        let c = &t.update_channels;
        let channel = match self.settings.updates {
            UpdateChannel::Stable => c.stable,
            UpdateChannel::Beta => c.beta,
            UpdateChannel::Nightly => c.nightly,
            UpdateChannel::Off => c.off,
        };
        drawer.row(gfx, t.update_channel, Some(channel), None, false, Step::None);
        let available = flow.step == UpdateStep::Available;
        if let Some(release) = &flow.release {
            let value = format!("{}, {} MB", release_label(release), megabytes(release.size));
            drawer.row(gfx, t.available_version, Some(&value), None, available, if available { Step::Right } else { Step::None });
        }
        let (done, total) = flow.progress;
        let note = match flow.step {
            UpdateStep::Checking => t.update_checking.to_string(),
            UpdateStep::Current => t.update_current.to_string(),
            UpdateStep::Available => t.update_available_help.to_string(),
            UpdateStep::Downloading => fill(t.update_downloading, &[("done", &megabytes(done)), ("total", &megabytes(total))]),
            UpdateStep::Unpacking => fill(t.update_unpacking, &[("done", &done.to_string()), ("total", &total.to_string())]),
            UpdateStep::Installing if flow.status.is_empty() => t.update_installing.to_string(),
            _ => flow.status.clone(),
        };
        let lines = if flow.step == UpdateStep::Installing { 0 } else { 1 };
        drawer.note(gfx, &note, lines);
        if matches!(flow.step, UpdateStep::Downloading | UpdateStep::Unpacking) {
            // The bar above the note's line, as Rescan's.
            let y = drawer.bottom - gfx.line_height(crate::text::Role::Small) - 2.0 - 18.0 - 12.0;
            let bar = Rect::new(drawer.x, y, 240.0, 6.0);
            gfx.round_rect(bar, 3.0, 0.0, theme.line);
            if total > 0 {
                let filled = (240.0 * done.min(total) as f32 / total as f32).round();
                if filled > 0.0 {
                    gfx.round_rect(Rect::new(bar.x, y, filled, 6.0), 3.0, 0.0, theme.accent);
                }
            }
        }
        let confirm = Button::confirm(self.settings.confirm);
        let cancel = Button::cancel(self.settings.confirm);
        match flow.step {
            UpdateStep::Installing => {}
            UpdateStep::Checking | UpdateStep::Downloading | UpdateStep::Unpacking => {
                drawer.hints(gfx, &[(cancel, if flow.step == UpdateStep::Downloading { t.stop } else { t.close })]);
            }
            UpdateStep::Available => drawer.hints(gfx, &[(confirm, t.update_download), (cancel, t.update_later)]),
            UpdateStep::Ready => drawer.hints(gfx, &[(confirm, t.update_install), (cancel, t.close)]),
            UpdateStep::Updated => drawer.hints(gfx, &[(confirm, t.close)]),
            UpdateStep::Current | UpdateStep::Failed => drawer.hints(gfx, &[(confirm, t.check_again), (cancel, t.close)]),
        }
    }
}
