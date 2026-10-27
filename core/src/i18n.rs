//! The launcher's languages. Titles come from each title's
//! files and are not translated.

use crate::host::ScanPhase;
use crate::library::categories::{Category, CategoryName};
use crate::library::settings::Language;
use crate::locales::{Messages, DE, EN, ES, FR, TR};

/// Each language under its own name, in menu order.
pub const LANGUAGES: [(Language, &str); 5] = [
    (Language::En, "English"),
    (Language::Tr, "Türkçe"),
    (Language::De, "Deutsch"),
    (Language::Fr, "Français"),
    (Language::Es, "Español"),
];

/// The texts of a language.
pub fn messages(language: Language) -> &'static Messages {
    match language {
        Language::En => &EN,
        Language::Tr => &TR,
        Language::De => &DE,
        Language::Fr => &FR,
        Language::Es => &ES,
    }
}

/// A category's label in the language: a built-in one by its key, one the
/// user made as typed (labels are never stored translated).
pub fn category_label(t: &Messages, category: &Category) -> String {
    let c = &t.categories;
    match category.name {
        Some(CategoryName::Recent) => c.recent,
        Some(CategoryName::Favorites) => c.favorites,
        Some(CategoryName::Search) => c.search,
        Some(CategoryName::Games) => c.games,
        Some(CategoryName::System) => c.system,
        Some(CategoryName::Homebrew) => c.homebrew,
        Some(CategoryName::Psm) => c.psm,
        Some(CategoryName::Psp) => c.psp,
        Some(CategoryName::Psx) => c.psx,
        None => return category.label.clone(),
    }
    .to_string()
}

/// What a running scan is reading, in the language.
pub fn scan_phase(t: &Messages, phase: ScanPhase) -> &'static str {
    let p = &t.scan_phases;
    match phase {
        ScanPhase::Apps => p.apps,
        ScanPhase::System => p.system,
        ScanPhase::Psm => p.psm,
        ScanPhase::Psp => p.psp,
        ScanPhase::Images => p.images,
        ScanPhase::Retro => p.retro,
    }
}

/// `template` with each `{name}` replaced by its value; a place with no
/// value stays as it is.
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) if after[..close].chars().all(|c| c.is_alphanumeric() || c == '_') && close > 0 => {
                let name = &after[..close];
                match values.iter().find(|(key, _)| *key == name) {
                    Some((_, value)) => out.push_str(value),
                    None => {
                        out.push('{');
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[close + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Upper case for a heading. Turkish has a dotted and a dotless i, each with
/// its own capital.
pub fn upper(text: &str, language: Language) -> String {
    if language == Language::Tr {
        text.chars()
            .map(|c| match c {
                'i' => "İ".to_string(),
                'ı' => "I".to_string(),
                _ => c.to_uppercase().collect(),
            })
            .collect()
    } else {
        text.to_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `{name}` places of a text, sorted.
    fn places(text: &str) -> Vec<String> {
        let mut found: Vec<String> = text.split('{').skip(1).filter_map(|part| part.split_once('}')).map(|(name, _)| name.to_string()).collect();
        found.sort();
        found
    }

    #[test]
    fn every_language_has_the_same_places_as_english() {
        let english = messages(Language::En).texts();
        for (language, name) in LANGUAGES {
            let texts = messages(language).texts();
            assert_eq!(texts.len(), english.len());
            for ((key, text), (_, model)) in texts.iter().zip(&english) {
                assert_eq!(places(text), places(model), "{name}: {key}");
            }
        }
    }

    #[test]
    fn no_text_is_left_empty() {
        for (language, name) in LANGUAGES {
            for (key, text) in messages(language).texts() {
                assert!(!text.trim().is_empty(), "{name}: {key}");
            }
        }
    }

    #[test]
    fn fill_replaces_the_places_it_has_a_value_for() {
        assert_eq!(fill("Launching {title}...", &[("title", "Gravity Daze")]), "Launching Gravity Daze...");
        assert_eq!(fill("{count} found", &[("count", "3")]), "3 found");
        assert_eq!(fill("Press {button} again", &[]), "Press {button} again");
    }

    #[test]
    fn upper_keeps_the_turkish_i_apart() {
        assert_eq!(upper("Kategoriler", Language::Tr), "KATEGORİLER");
        assert_eq!(upper("Yazı tipi", Language::Tr), "YAZI TİPİ");
        assert_eq!(upper("Kategorien", Language::De), "KATEGORIEN");
    }

    #[test]
    fn psx_is_psx_in_every_language() {
        for (language, _) in LANGUAGES {
            assert_eq!(messages(language).categories.psx, "PSX");
        }
    }
}
