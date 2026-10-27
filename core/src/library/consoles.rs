//! Categories of RetroArch games, one per console.
//!
//! RetroArch names a console by its database (`Nintendo - Super Nintendo
//! Entertainment System`). The ids are saved (tab order, hidden tabs, a
//! title moved into one), so an id never changes once given out. A category
//! the user made earlier with the same id keeps it.

use super::categories::{slug, Category, CATEGORY_LABEL_MAX, PSP_ID, PSX_ID};

/// Database names with a short label: database, id after `retro-`, label.
/// Several databases may share a category.
const KNOWN: &[(&str, &str, &str)] = &[
    ("Nintendo - Nintendo Entertainment System", "nes", "NES"),
    ("Nintendo - Family Computer Disk System", "fds", "Famicom Disk"),
    ("Nintendo - Super Nintendo Entertainment System", "snes", "SNES"),
    ("Nintendo - Satellaview", "satellaview", "Satellaview"),
    ("Nintendo - Sufami Turbo", "sufami", "Sufami Turbo"),
    ("Nintendo - Nintendo 64", "n64", "N64"),
    ("Nintendo - Nintendo 64DD", "n64dd", "N64DD"),
    ("Nintendo - Game Boy", "gb", "Game Boy"),
    ("Nintendo - Game Boy Color", "gbc", "Game Boy Color"),
    ("Nintendo - Game Boy Advance", "gba", "GBA"),
    ("Nintendo - Nintendo DS", "nds", "Nintendo DS"),
    ("Nintendo - Virtual Boy", "vb", "Virtual Boy"),
    ("Nintendo - Pokemon Mini", "pokemini", "Pokemon Mini"),
    ("Sega - SG-1000", "sg1000", "SG-1000"),
    ("Sega - Master System - Mark III", "sms", "Master System"),
    ("Sega - Game Gear", "gg", "Game Gear"),
    ("Sega - Mega Drive - Genesis", "md", "Mega Drive"),
    ("Sega - Mega-CD - Sega CD", "mcd", "Mega-CD"),
    ("Sega - 32X", "32x", "32X"),
    ("Sega - PICO", "pico", "Sega Pico"),
    ("Sega - Saturn", "saturn", "Saturn"),
    ("Sega - Dreamcast", "dc", "Dreamcast"),
    ("NEC - PC Engine - TurboGrafx 16", "pce", "PC Engine"),
    ("NEC - PC Engine CD - TurboGrafx-CD", "pcecd", "PC Engine CD"),
    ("NEC - PC Engine SuperGrafx", "sgx", "SuperGrafx"),
    ("NEC - PC-FX", "pcfx", "PC-FX"),
    ("SNK - Neo Geo", "neogeo", "Neo Geo"),
    // A playlist the user named, as RetroArch's playlist manager lets them.
    ("Neo Geo", "neogeo", "Neo Geo"),
    ("SNK - Neo Geo CD", "neogeocd", "Neo Geo CD"),
    ("SNK - Neo Geo Pocket", "ngp", "NG Pocket"),
    ("SNK - Neo Geo Pocket Color", "ngpc", "NG Pocket Color"),
    ("FBNeo - Arcade Games", "arcade", "Arcade"),
    ("Atari - 2600", "a2600", "Atari 2600"),
    ("Atari - 5200", "a5200", "Atari 5200"),
    ("Atari - 7800", "a7800", "Atari 7800"),
    ("Atari - Lynx", "lynx", "Lynx"),
    ("Atari - Jaguar", "jaguar", "Jaguar"),
    ("Atari - ST", "atarist", "Atari ST"),
    ("Bandai - WonderSwan", "ws", "WonderSwan"),
    ("Bandai - WonderSwan Color", "wsc", "WonderSwan Color"),
    ("Coleco - ColecoVision", "coleco", "ColecoVision"),
    ("Mattel - Intellivision", "intv", "Intellivision"),
    ("GCE - Vectrex", "vectrex", "Vectrex"),
    ("Magnavox - Odyssey2", "odyssey2", "Odyssey 2"),
    ("Fairchild - Channel F", "channelf", "Channel F"),
    ("Watara - Supervision", "supervision", "Supervision"),
    ("The 3DO Company - 3DO", "3do", "3DO"),
    ("Microsoft - MSX", "msx", "MSX"),
    ("Microsoft - MSX2", "msx", "MSX"),
    ("Commodore - 64", "c64", "C64"),
    ("Commodore - Amiga", "amiga", "Amiga"),
    ("Commodore - VIC-20", "vic20", "VIC-20"),
    ("Sinclair - ZX Spectrum", "zxspectrum", "ZX Spectrum"),
    ("Sinclair - ZX 81", "zx81", "ZX81"),
    ("Amstrad - CPC", "cpc", "Amstrad CPC"),
    ("DOS", "dos", "DOS"),
    ("ScummVM", "scummvm", "ScummVM"),
    ("Sharp - X68000", "x68000", "X68000"),
    ("NEC - PC-98", "pc98", "PC-98"),
];

/// The category a console's games go to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConsoleCategory {
    pub id: String,
    /// Empty for the built-in PSX and PSP.
    pub label: String,
    /// The built-in PSX or PSP category.
    pub builtin: bool,
}

fn known(id: &str, label: &str) -> ConsoleCategory {
    ConsoleCategory { id: format!("retro-{id}"), label: label.into(), builtin: false }
}

/// A known console's category; RetroArch's PlayStation and PSP games join
/// the built-in tabs; every MAME version shares FBNeo's arcade tab. An
/// unknown console gets `retro-` and its name as a slug, labeled with the
/// part of its name after the maker.
pub fn console_category(database: &str) -> ConsoleCategory {
    let name = database.trim();
    let shared = match name {
        "Sony - PlayStation" => Some(PSX_ID),
        "Sony - PlayStation Portable" => Some(PSP_ID),
        _ => None,
    };
    if let Some(id) = shared {
        return ConsoleCategory { id: id.into(), label: String::new(), builtin: true };
    }
    if let Some((_, id, label)) = KNOWN.iter().find(|(db, _, _)| *db == name) {
        return known(id, label);
    }
    if name.starts_with("MAME") {
        return known("arcade", "Arcade");
    }
    let parts: Vec<&str> = name.split(" - ").collect();
    let after_maker = if parts.len() > 1 { parts[1..].join(" - ") } else { name.to_string() };
    // Cut first, then trimmed, as Pocket Shelf 0.1 did.
    let label: String = after_maker.chars().take(CATEGORY_LABEL_MAX).collect::<String>().trim().to_string();
    let mut id = format!("retro-{}", slug(name));
    id.truncate(32);
    let id = id.trim_end_matches('-').to_string();
    ConsoleCategory { id, label: if label.is_empty() { "RetroArch".into() } else { label }, builtin: false }
}

/// The categories RetroArch games need, one per id, in the order of their
/// first game; the built-in PSX and PSP are not among them.
pub fn console_categories<'a>(consoles: impl IntoIterator<Item = &'a str>) -> Vec<Category> {
    let mut result: Vec<Category> = Vec::new();
    for name in consoles {
        let category = console_category(name);
        if category.builtin || result.iter().any(|c| c.id == category.id) {
            continue;
        }
        result.push(Category { id: category.id, label: category.label, name: None, custom: false, smart: false, console: true });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::categories::{builtin_categories, order_categories, smart_categories, valid_category_id, SYSTEM_ID};
    use super::*;

    #[test]
    fn known_consoles_have_a_short_label_and_fixed_id() {
        assert_eq!(console_category("Nintendo - Super Nintendo Entertainment System"), known("snes", "SNES"));
        assert_eq!(console_category("Sega - Mega Drive - Genesis"), known("md", "Mega Drive"));
    }

    #[test]
    fn every_arcade_database_shares_one_category() {
        let arcade = console_category("FBNeo - Arcade Games").id;
        assert_eq!(console_category("MAME 2003-Plus").id, arcade);
        assert_eq!(console_category("MAME 2000").id, arcade);
    }

    #[test]
    fn playstation_and_psp_go_to_the_builtin_tabs() {
        let psx = console_category("Sony - PlayStation");
        assert!(psx.builtin && psx.id == PSX_ID);
        let psp = console_category("Sony - PlayStation Portable");
        assert!(psp.builtin && psp.id == PSP_ID);
    }

    #[test]
    fn unknown_console_is_named_after_the_part_following_its_maker() {
        let category = console_category("Acme - Super Handheld Deluxe Edition Plus");
        assert_eq!(category.label, "Super Handheld D");
        assert!(valid_category_id(&category.id));
        assert!(category.id.starts_with("retro-acme-super"));
        assert_eq!(console_category("Homebrew Things"), known("homebrew-things", "Homebrew Things"));
    }

    #[test]
    fn known_ids_are_valid() {
        for (db, _, _) in KNOWN {
            let category = console_category(db);
            assert!(valid_category_id(&category.id), "{db}");
            assert!(category.label.chars().count() <= CATEGORY_LABEL_MAX, "{db}");
        }
    }

    #[test]
    fn console_categories_lists_each_once_without_builtins() {
        let list = console_categories([
            "Nintendo - Game Boy",
            "MAME 2003-Plus",
            "Sony - PlayStation",
            "Nintendo - Game Boy",
            "FBNeo - Arcade Games",
        ]);
        let ids: Vec<&str> = list.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["retro-gb", "retro-arcade"]);
        assert!(list.iter().all(|c| c.console));
    }

    fn all() -> Vec<Category> {
        let consoles = console_categories(["Nintendo - Super Nintendo Entertainment System", "Sega - Mega Drive - Genesis"]);
        let mut all = smart_categories();
        all.extend(builtin_categories());
        all.extend(consoles);
        all.push(Category::custom("emulators", "Emulators"));
        all
    }

    fn ids(saved: &[String]) -> Vec<String> {
        order_categories(&all(), saved).into_iter().map(|c| c.id).collect()
    }

    fn at(order: &[String], id: &str) -> usize {
        order.iter().position(|x| x == id).unwrap()
    }

    #[test]
    fn with_nothing_saved_system_is_last() {
        let order = ids(&[]);
        assert_eq!(order.last().unwrap(), SYSTEM_ID);
        assert!(at(&order, "retro-snes") > at(&order, "builtin-psx"));
        assert!(at(&order, "emulators") > at(&order, "retro-md"));
        assert_eq!(&order[..2], ["smart-recent", "smart-favorites"]);
    }

    #[test]
    fn a_later_console_goes_before_system_while_system_is_last() {
        let saved: Vec<String> = ids(&[]).into_iter().filter(|id| id != "retro-md").collect();
        let order = ids(&saved);
        assert_eq!(order[order.len() - 1], SYSTEM_ID);
        assert_eq!(order[order.len() - 2], "retro-md");
    }

    #[test]
    fn system_moved_by_the_user_keeps_its_place() {
        let mut saved: Vec<String> = ids(&[]).into_iter().filter(|id| id != "retro-md" && id != SYSTEM_ID).collect();
        saved.insert(2, SYSTEM_ID.into());
        let order = ids(&saved);
        assert_eq!(order[2], SYSTEM_ID);
        assert_eq!(order.last().unwrap(), "retro-md");
    }
}
