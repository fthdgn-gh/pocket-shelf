//! The Vita platform modules, moved from the PocketJS host (docs/PLAN.md):
//! the title scan (`installed`, with `psm`, `pspemu`, `iso`, `appdb`,
//! `retroarch`), launching, pictures (`backdrop`, `art`, `dds`, `accent`) and
//! Adrenaline's plugin lists (`seplugins`, `adrenaline_config`), and HTTPS
//! (`http`, on the network stack from `net`). They changed
//! only where they handed textures to the JavaScript engine: they now return
//! RGBA pixels, which the app turns into textures.

pub mod accent;
pub mod adrenaline_config;
pub mod appdb;
pub mod art;
pub mod backdrop;
pub mod dds;
pub mod http;
pub mod ime;
pub mod installed;
pub mod iso;
pub mod net;
pub mod psm;
pub mod pspemu;
pub mod retroarch;
pub mod seplugins;
pub mod update;

/// The data folder, the same one Pocket Shelf 0.1 used.
pub fn data_dir() -> String {
    String::from(crate::host::DATA)
}

/// A line for the log: the system's debug output and the app's log file.
pub fn vita_log(args: std::fmt::Arguments<'_>) {
    crate::host::log_line(&args.to_string());
}
