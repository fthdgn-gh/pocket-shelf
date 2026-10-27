//! The Vita platform modules, moved from `hosts/vita/src/` (native/PLAN.md):
//! the title scan (`installed`, with `psm`, `pspemu`, `iso`, `appdb`,
//! `retroarch`), launching, pictures (`backdrop`, `art`, `dds`, `accent`) and
//! Adrenaline's plugin lists (`seplugins`, `adrenaline_config`). They changed
//! only where they handed textures to the JavaScript engine: they now return
//! RGBA pixels, which the app turns into textures.

pub mod accent;
pub mod adrenaline_config;
pub mod appdb;
pub mod art;
pub mod backdrop;
pub mod dds;
pub mod installed;
pub mod iso;
pub mod psm;
pub mod pspemu;
pub mod retroarch;
pub mod seplugins;

/// The current app's data folder (`hosts/vita/src/datafs.rs`).
pub fn data_dir() -> String {
    String::from(crate::host::DATA)
}

/// A line for the log: the system's debug output and the app's log file.
pub fn vita_log(args: std::fmt::Arguments<'_>) {
    crate::host::log_line(&args.to_string());
}
