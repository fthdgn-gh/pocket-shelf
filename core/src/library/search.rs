//! Title search: which titles a typed term finds, and in
//! what order.

use unicode_normalization::UnicodeNormalization;

/// Lowercase and without the marks on letters, so "pokemon" finds "Pokémon"
/// and "isik" finds "Işık". Letters that are not a base letter plus a mark
/// match their plain spelling (ı i, ß ss, æ ae, œ oe).
pub fn search_key(text: &str) -> String {
    let plain: String = text.nfd().filter(|ch| !('\u{300}'..='\u{36f}').contains(ch)).collect();
    let mut out = String::with_capacity(plain.len());
    for ch in plain.to_lowercase().chars() {
        match ch {
            'ı' => out.push('i'),
            'ß' => out.push_str("ss"),
            'æ' => out.push_str("ae"),
            'œ' => out.push_str("oe"),
            _ => out.push(ch),
        }
    }
    out
}

/// Indexes of the items `term` finds; `text` gives an item's title and id.
/// Every word has to be part of the title or the id. Titles that start with
/// the first word come first, then titles with a word that starts with it,
/// then the rest, each group in the items' order. An empty term finds
/// nothing.
pub fn search<T>(items: &[T], term: &str, text: impl Fn(&T) -> (&str, &str)) -> Vec<usize> {
    let key = search_key(term);
    let words: Vec<&str> = key.split_whitespace().collect();
    let Some(first) = words.first() else { return Vec::new() };
    let mut found: Vec<(u8, usize)> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let (title, id) = text(item);
        let title = search_key(title);
        let id = id.to_lowercase();
        if !words.iter().all(|word| title.contains(word) || id.contains(word)) {
            continue;
        }
        let rank = if title.starts_with(first) {
            0
        } else if title.split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit())).any(|part| part.starts_with(first)) {
            1
        } else {
            2
        };
        found.push((rank, index));
    }
    found.sort();
    found.into_iter().map(|(_, index)| index).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(items: &[(&str, &str)], term: &str) -> Vec<String> {
        search(items, term, |(id, title)| (*title, *id)).into_iter().map(|i| items[i].0.to_string()).collect()
    }

    const GAMES: &[(&str, &str)] = &[
        ("PCSE00317", "Castle Siege Tactics"),
        ("PCSE00444", "Iron Fist Arena"),
        ("PCSB00800", "Pokémon Stadium"),
        ("PCSF00042", "Star Harbor"),
        ("VITASHELL", "VitaShell"),
    ];

    #[test]
    fn an_empty_term_finds_nothing() {
        assert!(find(GAMES, "").is_empty());
        assert!(find(GAMES, "   ").is_empty());
    }

    #[test]
    fn starts_first_then_word_starts_then_the_rest() {
        assert_eq!(find(GAMES, "st"), ["PCSF00042", "PCSB00800", "PCSE00317", "PCSE00444"]);
    }

    #[test]
    fn case_and_marks_are_ignored() {
        assert_eq!(find(GAMES, "POKEMON"), ["PCSB00800"]);
        assert_eq!(find(GAMES, "pokémon"), ["PCSB00800"]);
    }

    #[test]
    fn letters_without_a_base_letter_match_their_plain_spelling() {
        let titles = &[("PCSB00001", "Işık Kılıcı"), ("PCSB00002", "Straße"), ("PCSB00003", "Œuvre"), ("PCSB00004", "İstanbul")];
        assert_eq!(find(titles, "isik kilici"), ["PCSB00001"]);
        assert_eq!(find(titles, "strasse"), ["PCSB00002"]);
        assert_eq!(find(titles, "oeuvre"), ["PCSB00003"]);
        assert_eq!(find(titles, "istanbul"), ["PCSB00004"]);
    }

    #[test]
    fn every_word_has_to_match_in_any_order() {
        assert_eq!(find(GAMES, "tactics castle"), ["PCSE00317"]);
        assert!(find(GAMES, "castle arena").is_empty());
    }

    #[test]
    fn a_title_id_matches_too() {
        assert_eq!(find(GAMES, "pcsf"), ["PCSF00042"]);
    }
}
