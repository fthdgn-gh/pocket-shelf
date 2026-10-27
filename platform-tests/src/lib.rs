//! The Vita platform modules with no system calls, included by path from
//! `vita/src/platform/` so their tests run on the desktop. RetroArch
//! playlists in `.samples/retroarch-playlists` (not in git) are read too when
//! `RETROARCH_SAMPLES` names the folder (`bun run test` sets it).

#[path = "../../vita/src/platform/accent.rs"]
pub mod accent;
#[path = "../../vita/src/platform/adrenaline_config.rs"]
pub mod adrenaline_config;
#[path = "../../vita/src/platform/dds.rs"]
pub mod dds;
#[path = "../../vita/src/platform/iso.rs"]
pub mod iso;
#[path = "../../vita/src/platform/retroarch.rs"]
pub mod retroarch;
#[path = "../../vita/src/platform/seplugins.rs"]
pub mod seplugins;
