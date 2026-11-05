//! The core recommended for each console on the Vita, in RetroArch and in
//! Emu4Vita++: the title editor's Core row offers it after Default
//! ([`RECOMMENDED`] in `cores.json`).
//!
//! Researched on 2026-11-05 for RetroArch 1.22.2's and Emu4Vita++ 0.81's
//! cores, full speed first and accuracy second. Only recommendations a
//! source backs are listed: GameTechWiki's "Emulators on PS Vita" (its
//! RetroArch core table and frame rates), the cores' Vita builds (gpSP's
//! and PCSX ReARMed's dynarecs), and Emu4Vita++'s core picker
//! (`arch/source/app.cpp`), which lists each console's cores in its order
//! and stars some (fceumm, genesis_plus_gx, snes9x2005_plus, chimerasnes,
//! gpsp). Arcade and Neo Geo are left out: the core depends on the ROM set.

use crate::library::settings::Emulator;

/// The `cores.json` value for "follow the recommendation".
pub const RECOMMENDED: &str = "recommended";

/// RetroArch's recommended core per console, as its file name.
const RETROARCH: &[(&str, &str)] = &[
    // Nestopia runs about 55-60 fps on the Vita; GameTechWiki says to use FCEUmm.
    ("Nintendo - Nintendo Entertainment System", "fceumm_libretro.self"),
    ("Nintendo - Family Computer Disk System", "fceumm_libretro.self"),
    // Full speed with the better sound; snes9x2010 runs about 40 fps.
    ("Nintendo - Super Nintendo Entertainment System", "snes9x2005_plus_libretro.self"),
    ("Nintendo - Game Boy", "gambatte_libretro.self"),
    ("Nintendo - Game Boy Color", "gambatte_libretro.self"),
    // Built with its dynarec on the Vita; mGBA and VBA Next are slower.
    ("Nintendo - Game Boy Advance", "gpsp_libretro.self"),
    ("Sega - Mega Drive - Genesis", "genesis_plus_gx_libretro.self"),
    ("Sega - Master System - Mark III", "genesis_plus_gx_libretro.self"),
    ("Sega - Game Gear", "genesis_plus_gx_libretro.self"),
    ("Sega - SG-1000", "genesis_plus_gx_libretro.self"),
    // Its dynarec and NEON renderer are built for the Vita.
    ("Sony - PlayStation", "pcsx_rearmed_libretro.self"),
    ("NEC - PC Engine - TurboGrafx 16", "mednafen_pce_fast_libretro.self"),
    ("NEC - PC Engine CD - TurboGrafx-CD", "mednafen_pce_fast_libretro.self"),
    // RACE is the least accurate.
    ("SNK - Neo Geo Pocket", "mednafen_ngp_libretro.self"),
    ("SNK - Neo Geo Pocket Color", "mednafen_ngp_libretro.self"),
    ("Atari - 2600", "stella2014_libretro.self"),
    ("Atari - 5200", "a5200_libretro.self"),
    // x64sc is the accurate, slower one.
    ("Commodore - 64", "vice_x64_libretro.self"),
    ("Microsoft - MSX", "bluemsx_libretro.self"),
    ("Microsoft - MSX2", "bluemsx_libretro.self"),
    ("DOS", "dosbox_pure_libretro.self"),
];

/// Emu4Vita++'s: the starred or first core of its picker.
const EMU4VITA: &[(&str, &str)] = &[
    ("Nintendo - Nintendo Entertainment System", "fceumm"),
    ("Nintendo - Family Computer Disk System", "fceumm"),
    ("Nintendo - Super Nintendo Entertainment System", "snes9x2005_plus"),
    ("Nintendo - Game Boy", "gambatte"),
    ("Nintendo - Game Boy Color", "gambatte"),
    ("Nintendo - Game Boy Advance", "gpsp"),
    ("Sega - Mega Drive - Genesis", "genesis_plus_gx"),
    ("Sega - Master System - Mark III", "genesis_plus_gx"),
    ("Sega - Game Gear", "genesis_plus_gx"),
    ("Sega - SG-1000", "genesis_plus_gx"),
    ("NEC - PC Engine - TurboGrafx 16", "mednafen_pce_fast"),
    ("NEC - PC Engine CD - TurboGrafx-CD", "mednafen_pce_fast"),
    ("Microsoft - MSX", "bluemsx"),
    ("Microsoft - MSX2", "bluemsx"),
];

/// The core recommended for a console in an app; None when there is no
/// recommendation (one core only, or the ROM set decides).
pub fn recommended(app: Emulator, console: &str) -> Option<&'static str> {
    let table = match app {
        Emulator::RetroArch => RETROARCH,
        Emulator::Emu4Vita => EMU4VITA,
    };
    table.iter().find(|(c, _)| *c == console).map(|(_, core)| *core)
}

/// The core a saved choice starts: itself, or for [`RECOMMENDED`] the
/// console's recommendation (None, the default, when there is none).
pub fn resolve<'a>(choice: &'a str, app: Emulator, console: &str) -> Option<&'a str> {
    if choice == RECOMMENDED {
        recommended(app, console)
    } else {
        Some(choice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_app_has_its_own_names_and_recommended_follows_the_table() {
        let snes = "Nintendo - Super Nintendo Entertainment System";
        assert_eq!(recommended(Emulator::RetroArch, snes), Some("snes9x2005_plus_libretro.self"));
        assert_eq!(recommended(Emulator::Emu4Vita, snes), Some("snes9x2005_plus"));
        assert_eq!(recommended(Emulator::RetroArch, "SNK - Neo Geo"), None, "the ROM set decides");
        assert_eq!(resolve(RECOMMENDED, Emulator::RetroArch, "Nintendo - Game Boy Advance"), Some("gpsp_libretro.self"));
        assert_eq!(resolve(RECOMMENDED, Emulator::RetroArch, "SNK - Neo Geo"), None);
        assert_eq!(resolve("mgba_libretro.self", Emulator::RetroArch, "Nintendo - Game Boy Advance"), Some("mgba_libretro.self"));
        for (_, core) in RETROARCH {
            assert!(core.ends_with("_libretro.self"), "{core}");
        }
    }
}
