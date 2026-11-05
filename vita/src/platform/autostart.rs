//! SELECT ▸ Console start's file work (`shelf_core::autostart`) on a worker
//! thread: the tai config in use (`ux0:tai/config.txt`, else
//! `ur0:tai/config.txt`, as taiHEN reads them), Pocket Shelf's plugin in
//! `ur0:tai`, and AutoBoot's `boot.cfg`.
//!
//! A changed tai config is checked to differ from the old one only in
//! Pocket Shelf's line, the old one is kept as `<config>.pocketshelf.bak`,
//! the new one is written to `<config>.part` and renamed over it, then read
//! back. Holding L while the console starts skips every plugin, should the
//! plugin keep the home screen from starting.

use std::fs;
use std::sync::mpsc::Sender;

use shelf_core::autostart::{
    autoboot_title, find_plugins, only_ours_changed, with_plugin, without_plugin, Change, Setup, AUTOBOOT_CFG, OUR_PLUGIN_PATH, OUR_PLUGIN_SOURCE,
    PLUGIN_LOG, TAI_CONFIGS,
};
use shelf_core::host::Event;

use crate::host::{log_line, DATA};

pub fn run(change: Change, sender: Sender<Event>) {
    let result = match &change {
        Change::Check => refresh_plugin(),
        Change::Install => install(),
        Change::Remove => remove(),
        Change::AutoBoot(title) => set_autoboot(title),
    };
    let mut setup = read_setup();
    if let Err(error) = result {
        log_line(&format!("console start: {change:?} failed: {error}"));
        setup.error = error;
    }
    log_line(&format!("console start: {change:?} -> {setup:?}"));
    if change == Change::Check {
        plugin_log();
    }
    let _ = sender.send(Event::AutoStart(setup));
}

/// The tai config taiHEN reads: the first that exists.
fn config_path() -> Option<&'static str> {
    TAI_CONFIGS.iter().copied().find(|path| fs::metadata(path).is_ok_and(|m| m.is_file()))
}

fn read_text(path: &str) -> Result<String, String> {
    fs::read(path).map(|b| String::from_utf8_lossy(&b).into_owned()).map_err(|e| format!("{path}: {e}"))
}

fn read_setup() -> Setup {
    let Some(config) = config_path() else { return Setup::default() };
    let text = read_text(config).unwrap_or_default();
    let title = fs::read(AUTOBOOT_CFG).map(|b| autoboot_title(&b)).unwrap_or_default();
    Setup { config: config.to_string(), found: find_plugins(&text), autoboot_title: title, error: String::new() }
}

/// Write `bytes` to `path` through `<path>.part`.
fn write_atomic(path: &str, bytes: &[u8]) -> Result<(), String> {
    let part = format!("{path}.part");
    fs::write(&part, bytes).map_err(|e| format!("{part}: {e}"))?;
    fs::rename(&part, path).map_err(|e| format!("{path}: {e}"))
}

/// The installed plugin replaced by this build's when they differ (an
/// update of Pocket Shelf brings a new plugin).
fn refresh_plugin() -> Result<(), String> {
    if !read_setup().found.ours {
        return Ok(());
    }
    let ours = fs::read(OUR_PLUGIN_SOURCE).map_err(|e| format!("{OUR_PLUGIN_SOURCE}: {e}"))?;
    if fs::read(OUR_PLUGIN_PATH).ok().as_deref() != Some(&ours[..]) {
        write_atomic(OUR_PLUGIN_PATH, &ours)?;
        log_line(&format!("console start: {OUR_PLUGIN_PATH} updated ({} bytes)", ours.len()));
    }
    Ok(())
}

/// The tai config changed by `edit`, written when it differs and changes
/// nothing but Pocket Shelf's line.
fn change_config(edit: fn(&str) -> String) -> Result<(), String> {
    let path = config_path().ok_or_else(|| "no tai config".to_string())?;
    let original = read_text(path)?;
    let changed = edit(&original);
    if changed == original {
        return Ok(());
    }
    if !only_ours_changed(&original, &changed) {
        return Err("the change would touch other lines".into());
    }
    fs::write(format!("{path}.pocketshelf.bak"), &original).map_err(|e| format!("{path}.pocketshelf.bak: {e}"))?;
    write_atomic(path, changed.as_bytes())?;
    if read_text(path)? != changed {
        // Put the old one back rather than leave a config that is neither.
        let _ = write_atomic(path, original.as_bytes());
        return Err(format!("{path} did not read back as written"));
    }
    log_line(&format!("console start: {path} changed ({} -> {} bytes)", original.len(), changed.len()));
    Ok(())
}

fn install() -> Result<(), String> {
    let ours = fs::read(OUR_PLUGIN_SOURCE).map_err(|e| format!("{OUR_PLUGIN_SOURCE}: {e}"))?;
    let _ = fs::create_dir_all("ur0:tai");
    write_atomic(OUR_PLUGIN_PATH, &ours)?;
    change_config(with_plugin)
}

fn remove() -> Result<(), String> {
    change_config(without_plugin)?;
    let _ = fs::remove_file(OUR_PLUGIN_PATH);
    Ok(())
}

/// AutoBoot's title: nine bytes, as it reads them; empty deletes the file.
fn set_autoboot(title: &str) -> Result<(), String> {
    if title.is_empty() {
        let _ = fs::remove_file(AUTOBOOT_CFG);
        return Ok(());
    }
    let _ = fs::create_dir_all("ux0:data/AutoBoot");
    write_atomic(AUTOBOOT_CFG, title.as_bytes())
}

/// The plugin's log of this console start, into the app's log.
fn plugin_log() {
    let Ok(text) = read_text(&format!("{DATA}/{PLUGIN_LOG}")) else { return };
    for line in text.lines().rev().take(40).collect::<Vec<_>>().into_iter().rev() {
        log_line(&format!("[boot plugin] {line}"));
    }
}
