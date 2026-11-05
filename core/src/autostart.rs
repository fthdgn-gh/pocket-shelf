//! SELECT ▸ Console start: Pocket Shelf opened at console start, and again
//! when a game it started closes. Both are done by a taiHEN plugin in the
//! home screen (SceShell, the tai config's `*main` section): Pocket Shelf's
//! own (`shell-boot/`, shipped in the VPK as `tai/pocketshelf_boot.suprx`),
//! or for console start only, Rinnegatamante's AutoBoot, set to start
//! `POCKTSHLF` (`ux0:data/AutoBoot/boot.cfg`).
//!
//! The settings are `autostart.txt` ([`AutoStart`]), which Pocket Shelf's
//! plugin reads; before a launch Pocket Shelf writes [`RETURN_FILE`], which
//! the plugin deletes once it has acted. The tai config is read and changed
//! here as text ([`find_plugins`], [`with_plugin`], [`without_plugin`]); the
//! host reads and writes the files on a thread ([`Host::autostart`]).
//!
//! [`Host::autostart`]: crate::host::Host::autostart

/// The settings Pocket Shelf's plugin reads: `boot=1` opens Pocket Shelf at
/// console start, `return=1` after a game it started. A file of its own,
/// in `key=value` lines the plugin's C code reads.
pub const AUTOSTART_FILE: &str = "autostart.txt";

/// Written before a launch when Return after a game is on; the plugin
/// deletes it.
pub const RETURN_FILE: &str = "return.txt";

/// The plugin's log, written at each console start (`boot-plugin.log`).
pub const PLUGIN_LOG: &str = "boot-plugin.log";

/// Pocket Shelf's plugin: its file name, where it is copied from in the
/// app's folder, and where it is installed.
pub const OUR_PLUGIN: &str = "pocketshelf_boot.suprx";
pub const OUR_PLUGIN_SOURCE: &str = "app0:tai/pocketshelf_boot.suprx";
pub const OUR_PLUGIN_PATH: &str = "ur0:tai/pocketshelf_boot.suprx";

/// AutoBoot's file name, and the file holding the title it starts (nine
/// bytes, the title id).
pub const AUTOBOOT_PLUGIN: &str = "autoboot.suprx";
pub const AUTOBOOT_CFG: &str = "ux0:data/AutoBoot/boot.cfg";

/// The tai configs, in the order taiHEN tries them: the first that exists
/// is the one in use.
pub const TAI_CONFIGS: [&str; 2] = ["ux0:tai/config.txt", "ur0:tai/config.txt"];

pub const SELF_ID: &str = "POCKTSHLF";

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct AutoStart {
    /// Pocket Shelf's plugin opens Pocket Shelf at console start.
    pub boot: bool,
    /// It opens it again when a game Pocket Shelf started closes.
    pub back: bool,
    /// The title AutoBoot started before Pocket Shelf set it to itself, put
    /// back when Open at console start is turned off (`autoboot=`).
    pub autoboot_before: String,
}

impl AutoStart {
    pub fn parse(bytes: &[u8]) -> AutoStart {
        let text = String::from_utf8_lossy(bytes);
        let on = |key: &str| text.lines().any(|line| line.trim() == format!("{key}=1"));
        let before = text.lines().find_map(|line| line.trim().strip_prefix("autoboot=")).unwrap_or("");
        AutoStart { boot: on("boot"), back: on("return"), autoboot_before: autoboot_title(before.as_bytes()) }
    }

    pub fn render(&self) -> String {
        let mut text = format!("boot={}\nreturn={}\n", u8::from(self.boot), u8::from(self.back));
        if !self.autoboot_before.is_empty() {
            text.push_str(&format!("autoboot={}\n", self.autoboot_before));
        }
        text
    }
}

/// The plugins in the `*main` section that matter here.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Found {
    pub ours: bool,
    pub autoboot: bool,
}

/// A line's plugin file name, lowercase: the part after the last `/` or
/// `:`. None for a section header, a comment or an empty line.
fn plugin_name(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') || line.starts_with('*') {
        return None;
    }
    Some(line.rsplit(['/', ':']).next().unwrap_or(line).to_lowercase())
}

/// Whether a line is the header of the home screen's section.
fn is_main(line: &str) -> bool {
    line.trim() == "*main"
}

/// Pocket Shelf's plugin and AutoBoot in a tai config's `*main` section.
pub fn find_plugins(config: &str) -> Found {
    let mut found = Found::default();
    let mut in_main = false;
    for line in config.lines() {
        if line.trim().starts_with('*') {
            in_main = is_main(line);
            continue;
        }
        if !in_main {
            continue;
        }
        match plugin_name(line).as_deref() {
            Some(OUR_PLUGIN) => found.ours = true,
            Some(AUTOBOOT_PLUGIN) => found.autoboot = true,
            _ => {}
        }
    }
    found
}

/// The config with Pocket Shelf's plugin as the first line of `*main` (a
/// `*main` section added at the end when there is none), every other line
/// as it was, in the file's own line endings. Unchanged when it is there.
pub fn with_plugin(config: &str) -> String {
    if find_plugins(config).ours {
        return config.to_string();
    }
    let newline = if config.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::with_capacity(config.len() + 64);
    let mut added = false;
    for line in config.split_inclusive('\n') {
        out.push_str(line);
        if !added && is_main(line) {
            if !line.ends_with('\n') {
                out.push_str(newline);
            }
            out.push_str(OUR_PLUGIN_PATH);
            out.push_str(newline);
            added = true;
        }
    }
    if !added {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(newline);
        }
        out.push_str("*main");
        out.push_str(newline);
        out.push_str(OUR_PLUGIN_PATH);
        out.push_str(newline);
    }
    out
}

/// The config without Pocket Shelf's plugin's lines in `*main`.
pub fn without_plugin(config: &str) -> String {
    let mut out = String::with_capacity(config.len());
    let mut in_main = false;
    for line in config.split_inclusive('\n') {
        if line.trim().starts_with('*') {
            in_main = is_main(line);
        } else if in_main && plugin_name(line).as_deref() == Some(OUR_PLUGIN) {
            continue;
        }
        out.push_str(line);
    }
    out
}

/// Whether `changed` is `original` with only Pocket Shelf's line (and a
/// `*main` header) added or removed: checked before a changed config is
/// written over the old one.
pub fn only_ours_changed(original: &str, changed: &str) -> bool {
    let rest = |text: &str| -> Vec<String> {
        text.lines().map(str::trim).filter(|l| !l.is_empty() && *l != "*main" && plugin_name(l).as_deref() != Some(OUR_PLUGIN)).map(String::from).collect()
    };
    rest(original) == rest(changed)
}

/// The title AutoBoot starts, from `boot.cfg`'s first nine bytes.
pub fn autoboot_title(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&bytes[..bytes.len().min(9)]).trim().to_string()
}

/// What the host found: the tai config in use and the plugins in it.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Setup {
    /// The tai config in use; empty when there is none (no taiHEN).
    pub config: String,
    pub found: Found,
    /// The title AutoBoot starts (empty when `boot.cfg` is missing).
    pub autoboot_title: String,
    /// The last change's error, for the drawer's note; empty when it worked.
    pub error: String,
}

impl Setup {
    /// Whether Pocket Shelf opens at console start: through AutoBoot when it
    /// is installed, else through Pocket Shelf's plugin.
    pub fn opens_at_start(&self, settings: &AutoStart) -> bool {
        if self.found.autoboot {
            self.autoboot_title == SELF_ID
        } else {
            self.found.ours && settings.boot
        }
    }
}

/// What the drawer asks the host to do.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Change {
    /// Read the tai config and AutoBoot's title; refresh an installed
    /// plugin whose file is older than the app's.
    Check,
    /// Copy Pocket Shelf's plugin to `ur0:tai` and add it to `*main`.
    Install,
    /// Take Pocket Shelf's plugin out of `*main` and delete its file.
    Remove,
    /// Write AutoBoot's title (`POCKTSHLF`, or the one it had before);
    /// empty deletes `boot.cfg`.
    AutoBoot(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "# taiHEN config\r\n*KERNEL\r\nur0:tai/nonpdrm.skprx\r\n*main\r\nux0:tai/AutoBoot.suprx\r\n*NPXS10015\r\nur0:tai/henkaku.suprx\r\n";

    #[test]
    fn settings_round_trip_as_the_plugin_reads_them() {
        let both = AutoStart { boot: true, back: true, ..Default::default() };
        assert_eq!(both.render(), "boot=1\nreturn=1\n");
        assert_eq!(AutoStart::parse(both.render().as_bytes()), both);
        assert_eq!(AutoStart::parse(b"boot=0\nreturn=1\n"), AutoStart { back: true, ..Default::default() });
        let before = AutoStart { autoboot_before: "PCSB00245".into(), ..Default::default() };
        assert_eq!(before.render(), "boot=0\nreturn=0\nautoboot=PCSB00245\n");
        assert_eq!(AutoStart::parse(before.render().as_bytes()), before);
        assert_eq!(AutoStart::parse(b""), AutoStart::default());
    }

    #[test]
    fn the_plugin_goes_first_in_main_and_comes_out_again() {
        assert_eq!(find_plugins(CONFIG), Found { ours: false, autoboot: true });
        let added = with_plugin(CONFIG);
        assert!(added.contains("*main\r\nur0:tai/pocketshelf_boot.suprx\r\nux0:tai/AutoBoot.suprx\r\n"), "{added:?}");
        assert_eq!(find_plugins(&added), Found { ours: true, autoboot: true });
        assert!(only_ours_changed(CONFIG, &added));
        assert_eq!(with_plugin(&added), added, "once");
        assert_eq!(without_plugin(&added), CONFIG);

        // No *main: a section at the end, in the file's line endings.
        let plain = "*KERNEL\nur0:tai/a.skprx";
        let added = with_plugin(plain);
        assert_eq!(added, "*KERNEL\nur0:tai/a.skprx\n*main\nur0:tai/pocketshelf_boot.suprx\n");
        assert!(only_ours_changed(plain, &added));
        assert_eq!(with_plugin(""), "*main\nur0:tai/pocketshelf_boot.suprx\n");

        // A plugin of that name in another section, or commented out, is
        // not ours in *main; and a change to another line is caught.
        assert!(!find_plugins("*NPXS10015\nur0:tai/pocketshelf_boot.suprx\n*main\n# ur0:tai/pocketshelf_boot.suprx\n").ours);
        assert!(!only_ours_changed(CONFIG, &CONFIG.replace("nonpdrm", "other")));
    }

    #[test]
    fn autoboot_decides_console_start_when_it_is_installed() {
        assert_eq!(autoboot_title(b"PCSB00245"), "PCSB00245");
        assert_eq!(autoboot_title(b"POCKTSHLF\n"), "POCKTSHLF");
        let on = AutoStart { boot: true, ..Default::default() };
        let autoboot = Setup { found: Found { ours: true, autoboot: true }, autoboot_title: "PCSB00245".into(), ..Default::default() };
        assert!(!autoboot.opens_at_start(&on));
        assert!(Setup { autoboot_title: SELF_ID.into(), ..autoboot.clone() }.opens_at_start(&AutoStart::default()));
        let ours = Setup { found: Found { ours: true, autoboot: false }, ..Default::default() };
        assert!(ours.opens_at_start(&on) && !ours.opens_at_start(&AutoStart::default()));
    }
}
