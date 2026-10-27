//! The on-screen keyboard's keys and state (`src/keyboard.ts`, the
//! keyboard part of `src/state.ts`): letters, symbols and accents pages, a
//! one-press shift, and what the typed text is for.

use crate::library::settings::Language;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Char,
    Shift,
    Symbols,
    Space,
    Delete,
    Done,
}

/// One key: a character with its shifted twin, or a wide action key.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Key {
    pub action: Action,
    pub lower: char,
    pub upper: char,
}

impl Key {
    pub fn wide(self) -> bool {
        self.action != Action::Char
    }
}

const fn action(action: Action) -> Key {
    Key { action, lower: ' ', upper: ' ' }
}

const BOTTOM_ROW: [Key; 5] = [action(Action::Shift), action(Action::Symbols), action(Action::Space), action(Action::Delete), action(Action::Done)];

fn chars(lower: &str, upper: &str) -> Vec<Key> {
    lower.chars().zip(upper.chars()).map(|(lower, upper)| Key { action: Action::Char, lower, upper }).collect()
}

/// The keyboard's pages, in the order the page key steps through them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Letters,
    Symbols,
    Accents,
}

pub fn letter_rows() -> Vec<Vec<Key>> {
    vec![
        chars("1234567890", "!@#$%&*()?"),
        chars("qwertyuiop", "QWERTYUIOP"),
        chars("asdfghjkl-", "ASDFGHJKL_"),
        chars("zxcvbnm.,'", "ZXCVBNM:;\""),
        BOTTOM_ROW.to_vec(),
    ]
}

pub fn symbol_rows() -> Vec<Vec<Key>> {
    vec![
        chars("!@#$%^&*()", "!@#$%^&*()"),
        chars("-_=+[]{}<>", "-_=+[]{}<>"),
        chars(";:'\",./?\\|", ";:'\",./?\\|"),
        chars("~`", "~`"),
        BOTTOM_ROW.to_vec(),
    ]
}

/// Letters with marks and the other letters the four translated languages
/// use, `[lower, upper]` in layout order. `i` is here for its Turkish
/// capital; `ß`, `¿` and `¡` have no capital.
const ACCENTS: [(char, char); 30] = [
    ('á', 'Á'), ('à', 'À'), ('â', 'Â'), ('ä', 'Ä'), ('æ', 'Æ'), ('ç', 'Ç'), ('é', 'É'), ('è', 'È'), ('ê', 'Ê'), ('ë', 'Ë'),
    ('í', 'Í'), ('î', 'Î'), ('ï', 'Ï'), ('ı', 'I'), ('i', 'İ'), ('ñ', 'Ñ'), ('ó', 'Ó'), ('ô', 'Ô'), ('ö', 'Ö'), ('œ', 'Œ'),
    ('ú', 'Ú'), ('ù', 'Ù'), ('û', 'Û'), ('ü', 'Ü'), ('ÿ', 'Ÿ'), ('ğ', 'Ğ'), ('ş', 'Ş'), ('ß', 'ß'), ('¿', '¿'), ('¡', '¡'),
];

/// The letters each language types most, which lead its accents page.
pub fn accents_first(language: Language) -> &'static str {
    match language {
        Language::Tr => "çğıiöşü",
        Language::De => "äöüß",
        Language::Fr => "éèêëàâçîïôùûüÿœæ",
        Language::Es => "áéíóúñü¿¡",
        Language::En => "",
    }
}

/// The accents page: the same thirty keys in every language, its own
/// letters first, three rows of ten under the digits.
pub fn accent_rows(language: Language) -> Vec<Vec<Key>> {
    let first: Vec<char> = accents_first(language).chars().collect();
    let rank = |c: char| first.iter().position(|&f| f == c).unwrap_or(first.len());
    let mut keys: Vec<(usize, Key)> =
        ACCENTS.iter().enumerate().map(|(i, &(lower, upper))| (i, Key { action: Action::Char, lower, upper })).collect();
    keys.sort_by_key(|(i, key)| (rank(key.lower), *i));
    let keys: Vec<Key> = keys.into_iter().map(|(_, k)| k).collect();
    vec![letter_rows().remove(0), keys[0..10].to_vec(), keys[10..20].to_vec(), keys[20..30].to_vec(), BOTTOM_ROW.to_vec()]
}

/// The column in a row of `to` keys under column `col` of a row of `from`.
pub fn map_column(from: usize, to: usize, col: usize) -> usize {
    (to - 1).min(((col as f32 + 0.5) * to as f32 / from as f32) as usize)
}

/// What the typed text is for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Target {
    /// The edited title's name.
    Title,
    NewCategory,
    /// A category's new name.
    Category(String),
    /// SteamGridDB's search term (phase 5).
    Search,
    /// The SteamGridDB API key (phase 5).
    ApiKey,
    /// The library search (square).
    Library,
}

/// The longest SteamGridDB API key (`KEY_MAX` in `state.ts`).
pub const KEY_MAX: usize = 64;

/// The keyboard while open.
pub struct Keyboard {
    pub target: Target,
    pub text: String,
    pub row: usize,
    pub col: usize,
    pub shift: bool,
    pub page: Page,
    language: Language,
}

impl Keyboard {
    pub fn new(target: Target, text: &str, language: Language) -> Keyboard {
        Keyboard { target, text: text.to_string(), row: 1, col: 0, shift: false, page: Page::Letters, language }
    }

    pub fn rows(&self) -> Vec<Vec<Key>> {
        match self.page {
            Page::Letters => letter_rows(),
            Page::Symbols => symbol_rows(),
            Page::Accents => accent_rows(self.language),
        }
    }

    /// The most characters the target takes.
    pub fn limit(&self) -> usize {
        match self.target {
            Target::ApiKey => KEY_MAX,
            Target::Title | Target::Search | Target::Library => crate::library::overrides::TITLE_MAX,
            Target::NewCategory | Target::Category(_) => crate::library::categories::CATEGORY_LABEL_MAX,
        }
    }

    /// Move the highlight; up and down keep it under the same part of the
    /// row, and both wrap.
    pub fn step(&mut self, dx: i32, dy: i32) {
        let rows = self.rows();
        if dy != 0 {
            let next = (self.row as i32 + dy).rem_euclid(rows.len() as i32) as usize;
            self.col = map_column(rows[self.row].len(), rows[next].len(), self.col);
            self.row = next;
        }
        if dx != 0 {
            let count = rows[self.row].len() as i32;
            self.col = (self.col as i32 + dx).rem_euclid(count) as usize;
        }
    }

    fn type_text(&mut self, text: &str) {
        self.text.push_str(text);
        let limit = self.limit();
        if self.text.chars().count() > limit {
            self.text = self.text.chars().take(limit).collect();
        }
    }

    pub fn backspace(&mut self) {
        self.text.pop();
    }

    pub fn toggle_shift(&mut self) {
        self.shift = !self.shift;
    }

    /// The next page: letters, symbols, accents. The highlight keeps its row.
    pub fn next_page(&mut self) {
        let from = self.rows()[self.row].len();
        self.page = match self.page {
            Page::Letters => Page::Symbols,
            Page::Symbols => Page::Accents,
            Page::Accents => Page::Letters,
        };
        let to = self.rows()[self.row].len();
        self.col = map_column(from, to, self.col);
    }

    /// Press the highlighted key; true when it was Done.
    pub fn press(&mut self) -> bool {
        let key = self.rows()[self.row][self.col];
        match key.action {
            Action::Char => {
                // Shift lasts one character.
                let c = if self.shift { key.upper } else { key.lower };
                self.type_text(&c.to_string());
                self.shift = false;
            }
            Action::Space => self.type_text(" "),
            Action::Delete => self.backspace(),
            Action::Shift => self.toggle_shift(),
            Action::Symbols => self.next_page(),
            Action::Done => return true,
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages() -> Vec<Vec<Vec<Key>>> {
        let mut pages = vec![letter_rows(), symbol_rows()];
        for (language, _) in crate::i18n::LANGUAGES {
            pages.push(accent_rows(language));
        }
        pages
    }

    #[test]
    fn every_page_has_five_rows_and_shares_the_bottom_row() {
        for rows in pages() {
            assert_eq!(rows.len(), 5);
            assert_eq!(rows[4], BOTTOM_ROW.to_vec());
        }
        let actions: Vec<Action> = BOTTOM_ROW.iter().map(|k| k.action).collect();
        assert_eq!(actions, [Action::Shift, Action::Symbols, Action::Space, Action::Delete, Action::Done]);
    }

    #[test]
    fn map_column_keeps_the_highlight_under_the_same_part() {
        assert_eq!(map_column(10, 5, 0), 0);
        assert_eq!(map_column(10, 5, 9), 4);
        assert_eq!(map_column(5, 10, 2), 5);
        assert_eq!(map_column(10, 2, 9), 1);
        for rows in pages() {
            for from in &rows {
                for to in &rows {
                    for col in 0..from.len() {
                        assert!(map_column(from.len(), to.len(), col) < to.len());
                    }
                }
            }
        }
    }

    #[test]
    fn the_accents_page_has_the_same_thirty_letters_its_own_first() {
        let letters = |language| -> Vec<char> { accent_rows(language)[1..4].iter().flatten().map(|k| k.lower).collect() };
        let mut all = letters(Language::En);
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 30);
        for (language, _) in crate::i18n::LANGUAGES {
            let mut sorted = letters(language);
            sorted.sort();
            assert_eq!(sorted, all);
            let first: Vec<char> = accents_first(language).chars().collect();
            assert_eq!(letters(language)[..first.len()], first[..]);
        }
    }

    #[test]
    fn the_turkish_page_pairs_each_i_with_its_capital() {
        let keys: Vec<Key> = accent_rows(Language::Tr).into_iter().flatten().collect();
        assert_eq!(keys.iter().find(|k| k.lower == 'ı').unwrap().upper, 'I');
        assert_eq!(keys.iter().find(|k| k.lower == 'i').unwrap().upper, 'İ');
    }

    #[test]
    fn typing_shifts_once_and_stops_at_the_limit() {
        let mut keyboard = Keyboard::new(Target::NewCategory, "", Language::En);
        // Row 1 is qwerty: shift, then q, then w.
        keyboard.toggle_shift();
        keyboard.press();
        keyboard.step(1, 0);
        keyboard.press();
        assert_eq!(keyboard.text, "Qw");
        for _ in 0..20 {
            keyboard.press();
        }
        assert_eq!(keyboard.text.chars().count(), 16);
    }
}
