//! Pocket Shelf's on-screen keyboard, laid out and used like the Vita's
//! own: four rows of letters with Shift and backspace beside the bottom
//! letters, a page of digits and punctuation and one of other symbols, a
//! cursor in the text, and a letter's accents in a pop-up while ○ or a
//! finger is held on it.

use crate::library::settings::Language;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Char,
    /// One capital, or all capitals after a second press.
    Shift,
    Delete,
    /// The page key at the bottom left: digits from the letters, letters
    /// back from the other pages.
    Page,
    /// The key where Shift is on the letters: the symbols page from the
    /// digits, the digits back from the symbols.
    MorePage,
    Space,
    /// Move the cursor one character.
    Left,
    Right,
    Enter,
}

/// One key: a character with its shifted twin, or an action.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Key {
    pub action: Action,
    pub lower: char,
    pub upper: char,
}

/// A character key's width; the other keys' are `Key::width`. Ten keys and
/// their gaps fill the 832 pixel field.
pub const KEY_W: f32 = 76.0;
pub const KEY_GAP: f32 = 8.0;

impl Key {
    /// Width in pixels: Shift, backspace and the page keys one and a half
    /// keys, Enter two, the space bar what the bottom row leaves.
    pub fn width(self) -> f32 {
        match self.action {
            Action::Char | Action::Left | Action::Right => KEY_W,
            Action::Shift | Action::Delete | Action::Page | Action::MorePage => 116.0,
            Action::Enter => 160.0,
            Action::Space => 204.0,
        }
    }
}

const fn action(action: Action) -> Key {
    Key { action, lower: ' ', upper: ' ' }
}

const fn char_key(lower: char, upper: char) -> Key {
    Key { action: Action::Char, lower, upper }
}

fn chars(lower: &str, upper: &str) -> Vec<Key> {
    lower.chars().zip(upper.chars()).map(|(lower, upper)| char_key(lower, upper)).collect()
}

/// The keyboard's pages.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Letters,
    /// Digits and the common punctuation ("123").
    Digits,
    /// The other symbols ("#+=").
    Symbols,
}

/// The bottom row of every page: the page key, a comma, the space bar, a
/// full stop, the cursor keys and Enter.
fn bottom_row() -> Vec<Key> {
    vec![action(Action::Page), char_key(',', ','), action(Action::Space), char_key('.', '.'), action(Action::Left), action(Action::Right), action(Action::Enter)]
}

pub fn rows(page: Page) -> Vec<Vec<Key>> {
    match page {
        Page::Letters => vec![
            chars("qwertyuiop", "QWERTYUIOP"),
            chars("asdfghjkl", "ASDFGHJKL"),
            [action(Action::Shift)].into_iter().chain(chars("zxcvbnm", "ZXCVBNM")).chain([action(Action::Delete)]).collect(),
            bottom_row(),
        ],
        Page::Digits => vec![
            chars("1234567890", "1234567890"),
            chars("@#$%&*-+()", "@#$%&*-+()"),
            [action(Action::MorePage)].into_iter().chain(chars("!?'\":;/", "!?'\":;/")).chain([action(Action::Delete)]).collect(),
            bottom_row(),
        ],
        Page::Symbols => vec![
            chars("[]{}<>^~`|", "[]{}<>^~`|"),
            chars("\\=_€£¥°§", "\\=_€£¥°§"),
            [action(Action::MorePage)].into_iter().chain(chars("¡¿«»…", "¡¿«»…")).chain([action(Action::Delete)]).collect(),
            bottom_row(),
        ],
    }
}

/// Letters with marks and the other letters the four translated languages
/// use, `[lower, upper]`, each under the plain letter whose pop-up shows it.
/// `ı` and `i` are here for the Turkish capitals; `ß` has no capital.
const ACCENTS: [(char, char, char); 28] = [
    ('a', 'á', 'Á'), ('a', 'à', 'À'), ('a', 'â', 'Â'), ('a', 'ä', 'Ä'), ('a', 'æ', 'Æ'),
    ('c', 'ç', 'Ç'),
    ('e', 'é', 'É'), ('e', 'è', 'È'), ('e', 'ê', 'Ê'), ('e', 'ë', 'Ë'),
    ('i', 'í', 'Í'), ('i', 'î', 'Î'), ('i', 'ï', 'Ï'), ('i', 'ı', 'I'), ('i', 'i', 'İ'),
    ('n', 'ñ', 'Ñ'),
    ('o', 'ó', 'Ó'), ('o', 'ô', 'Ô'), ('o', 'ö', 'Ö'), ('o', 'œ', 'Œ'),
    ('u', 'ú', 'Ú'), ('u', 'ù', 'Ù'), ('u', 'û', 'Û'), ('u', 'ü', 'Ü'),
    ('y', 'ÿ', 'Ÿ'),
    ('g', 'ğ', 'Ğ'),
    ('s', 'ş', 'Ş'), ('s', 'ß', 'ß'),
];

/// The letters each language types most, which lead its pop-ups.
pub fn accents_first(language: Language) -> &'static str {
    match language {
        Language::Tr => "çğıiöşü",
        Language::De => "äöüß",
        Language::Fr => "éèêëàâçîïôùûüÿœæ",
        Language::Es => "áéíóúñü",
        Language::En => "",
    }
}

/// The accented forms of `base` (a plain lowercase letter), the language's
/// own first; empty for a letter with none.
pub fn accents_of(base: char, language: Language) -> Vec<Key> {
    let first: Vec<char> = accents_first(language).chars().collect();
    let rank = |c: char| first.iter().position(|&f| f == c).unwrap_or(first.len());
    let mut keys: Vec<(usize, Key)> =
        ACCENTS.iter().enumerate().filter(|(_, a)| a.0 == base).map(|(i, &(_, lower, upper))| (i, char_key(lower, upper))).collect();
    keys.sort_by_key(|(i, key)| (rank(key.lower), *i));
    keys.into_iter().map(|(_, k)| k).collect()
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
    /// SteamGridDB's search term.
    Search,
    /// The SteamGridDB API key.
    ApiKey,
    /// The library search (square).
    Library,
}

/// The longest SteamGridDB API key.
pub const KEY_MAX: usize = 64;

/// Shift's state: off, the next letter only, or every letter.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shift {
    Off,
    Once,
    Locked,
}

/// A letter's accents shown over it: the forms and the highlighted one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Accents {
    pub keys: Vec<Key>,
    pub index: usize,
}

/// The keyboard while open.
pub struct Keyboard {
    pub target: Target,
    /// The text as characters, so the cursor counts characters.
    chars: Vec<char>,
    /// Where the next character goes: 0 is before the first.
    pub cursor: usize,
    pub row: usize,
    pub col: usize,
    pub shift: Shift,
    pub page: Page,
    /// A letter's accents, while shown.
    pub accents: Option<Accents>,
    language: Language,
}

impl Keyboard {
    /// The keyboard over `text`, the cursor at its end, on the letters with
    /// `q` highlighted.
    pub fn new(target: Target, text: &str, language: Language) -> Keyboard {
        let chars: Vec<char> = text.chars().collect();
        let cursor = chars.len();
        Keyboard { target, chars, cursor, row: 0, col: 0, shift: Shift::Off, page: Page::Letters, accents: None, language }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    /// Replace the text, the cursor at its end.
    pub fn set_text(&mut self, text: &str) {
        self.chars = text.chars().collect();
        self.cursor = self.chars.len();
    }

    /// The text before and after the cursor.
    pub fn split(&self) -> (String, String) {
        (self.chars[..self.cursor].iter().collect(), self.chars[self.cursor..].iter().collect())
    }

    pub fn rows(&self) -> Vec<Vec<Key>> {
        rows(self.page)
    }

    pub fn key(&self) -> Key {
        let rows = self.rows();
        let row = &rows[self.row.min(rows.len() - 1)];
        row[self.col.min(row.len() - 1)]
    }

    /// Whether letters type as capitals.
    pub fn capitals(&self) -> bool {
        self.shift != Shift::Off
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
    /// row, and both wrap. With a letter's accents shown, left and right
    /// move through them.
    pub fn step(&mut self, dx: i32, dy: i32) {
        if let Some(accents) = &mut self.accents {
            let count = accents.keys.len() as i32;
            accents.index = (accents.index as i32 + dx).rem_euclid(count) as usize;
            return;
        }
        let rows = self.rows();
        if dy != 0 {
            let next = (self.row as i32 + dy).rem_euclid(rows.len() as i32) as usize;
            self.col = self.column_under(&rows[self.row], &rows[next]);
            self.row = next;
        }
        if dx != 0 {
            let count = rows[self.row].len() as i32;
            self.col = (self.col as i32 + dx).rem_euclid(count) as usize;
        }
    }

    /// The key in `to` under the middle of the highlighted key of `from`,
    /// by where the keys are drawn (their widths differ).
    fn column_under(&self, from: &[Key], to: &[Key]) -> usize {
        let middle = |keys: &[Key], col: usize| {
            let width: f32 = keys.iter().map(|k| k.width()).sum::<f32>() + KEY_GAP * (keys.len() - 1) as f32;
            let left: f32 = keys[..col].iter().map(|k| k.width() + KEY_GAP).sum();
            -width / 2.0 + left + keys[col].width() / 2.0
        };
        let at = middle(from, self.col.min(from.len() - 1));
        (0..to.len()).min_by(|&a, &b| (middle(to, a) - at).abs().total_cmp(&(middle(to, b) - at).abs())).unwrap_or(0)
    }

    /// Insert at the cursor, up to the target's limit.
    fn type_char(&mut self, c: char) {
        if self.chars.len() >= self.limit() {
            return;
        }
        self.chars.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// Delete the character before the cursor.
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
    }

    pub fn space(&mut self) {
        self.type_char(' ');
    }

    /// Move the cursor `delta` characters, within the text.
    pub fn move_cursor(&mut self, delta: i32) {
        self.cursor = (self.cursor as i32 + delta).clamp(0, self.chars.len() as i32) as usize;
    }

    /// Off, then the next letter, then every letter (a second press), then
    /// off.
    pub fn press_shift(&mut self) {
        self.shift = match self.shift {
            Shift::Off => Shift::Once,
            Shift::Once => Shift::Locked,
            Shift::Locked => Shift::Off,
        };
    }

    fn show_page(&mut self, page: Page) {
        let from = self.rows()[self.row].clone();
        self.page = page;
        let to = self.rows()[self.row].clone();
        self.col = self.column_under(&from, &to);
    }

    /// Press the highlighted key (or the highlighted accent); true for Enter.
    pub fn press(&mut self) -> bool {
        if let Some(accents) = self.accents.take() {
            let key = accents.keys[accents.index];
            self.type_letter(key);
            return false;
        }
        let key = self.key();
        match key.action {
            Action::Char => self.type_letter(key),
            Action::Space => self.space(),
            Action::Delete => self.backspace(),
            Action::Shift => self.press_shift(),
            Action::Page => self.show_page(if self.page == Page::Letters { Page::Digits } else { Page::Letters }),
            Action::MorePage => self.show_page(if self.page == Page::Digits { Page::Symbols } else { Page::Digits }),
            Action::Left => self.move_cursor(-1),
            Action::Right => self.move_cursor(1),
            Action::Enter => return true,
        }
        false
    }

    /// A character key: a capital while Shift is on; a single capital turns
    /// Shift off.
    fn type_letter(&mut self, key: Key) {
        self.type_char(if self.capitals() { key.upper } else { key.lower });
        if self.shift == Shift::Once {
            self.shift = Shift::Off;
        }
    }

    /// The highlighted letter held: its accents in place of the letter the
    /// press typed. False when it has none (the letter stays).
    pub fn hold(&mut self) -> bool {
        let key = self.key();
        if key.action != Action::Char || self.accents.is_some() {
            return false;
        }
        let keys = accents_of(key.lower, self.language);
        if keys.is_empty() {
            return false;
        }
        // The press typed the plain letter; the accent replaces it. A
        // capital typed with a single Shift brings Shift back for it.
        let typed = self.cursor > 0 && (self.chars[self.cursor - 1] == key.lower || self.chars[self.cursor - 1] == key.upper);
        if typed {
            if self.chars[self.cursor - 1] == key.upper && self.shift == Shift::Off && key.upper != key.lower {
                self.shift = Shift::Once;
            }
            self.backspace();
        }
        self.accents = Some(Accents { keys, index: 0 });
        true
    }

    /// Close a letter's accents without typing one.
    pub fn close_accents(&mut self) -> bool {
        self.accents.take().is_some()
    }

    /// Highlight a key by touch.
    pub fn focus(&mut self, row: usize, col: usize) {
        let rows = self.rows();
        self.row = row.min(rows.len() - 1);
        self.col = col.min(rows[self.row].len() - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGES: [Page; 3] = [Page::Letters, Page::Digits, Page::Symbols];

    fn row_width(keys: &[Key]) -> f32 {
        keys.iter().map(|k| k.width()).sum::<f32>() + KEY_GAP * (keys.len() - 1) as f32
    }

    #[test]
    fn every_page_has_four_rows_within_the_field_and_the_same_bottom_row() {
        for page in PAGES {
            let rows = rows(page);
            assert_eq!(rows.len(), 4);
            assert_eq!(rows[3], bottom_row());
            for row in &rows {
                assert!(row_width(row) <= 832.0, "{page:?} {}", row_width(row));
            }
        }
        // The bottom row fills the field.
        assert_eq!(row_width(&bottom_row()), 832.0);
    }

    #[test]
    fn every_character_is_in_figtree() {
        use fontdue::{Font, FontSettings};
        for bytes in [include_bytes!("../../../fonts/Figtree-Regular.ttf").as_slice(), include_bytes!("../../../fonts/Figtree-Bold.ttf").as_slice()] {
            let font = Font::from_bytes(bytes, FontSettings::default()).unwrap();
            let mut keys: Vec<Key> = PAGES.into_iter().flat_map(rows).flatten().collect();
            keys.extend(ACCENTS.iter().map(|&(_, lower, upper)| char_key(lower, upper)));
            for key in keys.iter().filter(|k| k.action == Action::Char) {
                for c in [key.lower, key.upper] {
                    assert_ne!(font.lookup_glyph_index(c), 0, "{c:?} is not in Figtree");
                }
            }
        }
    }

    #[test]
    fn typing_goes_in_at_the_cursor_and_backspace_takes_the_one_before_it() {
        let mut keyboard = Keyboard::new(Target::Title, "Gravty", Language::En);
        keyboard.move_cursor(-2);
        // Row 0 col 7 is `i`.
        keyboard.focus(0, 7);
        keyboard.press();
        assert_eq!(keyboard.text(), "Gravity");
        assert_eq!(keyboard.split(), ("Gravi".to_string(), "ty".to_string()));
        keyboard.backspace();
        assert_eq!(keyboard.text(), "Gravty");
        keyboard.move_cursor(-99);
        keyboard.backspace();
        assert_eq!((keyboard.text().as_str(), keyboard.cursor), ("Gravty", 0));
        keyboard.move_cursor(99);
        assert_eq!(keyboard.cursor, 6);
    }

    #[test]
    fn shift_capitalizes_one_letter_or_locks_on_a_second_press() {
        let mut keyboard = Keyboard::new(Target::NewCategory, "", Language::En);
        keyboard.press_shift();
        keyboard.focus(0, 0);
        keyboard.press();
        keyboard.press();
        assert_eq!(keyboard.text(), "Qq");
        keyboard.press_shift();
        keyboard.press_shift();
        keyboard.press();
        keyboard.press();
        assert_eq!(keyboard.text(), "QqQQ");
        keyboard.press_shift();
        keyboard.press();
        assert_eq!(keyboard.text(), "QqQQq");
        // The limit stops typing.
        for _ in 0..40 {
            keyboard.press();
        }
        assert_eq!(keyboard.text().chars().count(), keyboard.limit());
    }

    #[test]
    fn holding_a_letter_replaces_it_with_the_accent_chosen() {
        let mut keyboard = Keyboard::new(Target::Title, "Caf", Language::Fr);
        // Row 0 col 2 is `e`: pressed, then held.
        keyboard.focus(0, 2);
        keyboard.press();
        assert_eq!(keyboard.text(), "Cafe");
        assert!(keyboard.hold());
        assert_eq!(keyboard.text(), "Caf");
        // French leads with é.
        assert_eq!(keyboard.accents.as_ref().unwrap().keys[0].lower, 'é');
        keyboard.press();
        assert_eq!(keyboard.text(), "Café");
        assert!(keyboard.accents.is_none());
        // A letter with no accents keeps the letter typed.
        keyboard.focus(0, 1);
        keyboard.press();
        assert!(!keyboard.hold());
        assert_eq!(keyboard.text(), "Caféw");
    }

    #[test]
    fn turkish_i_keeps_its_own_capitals() {
        let i = accents_of('i', Language::Tr);
        assert_eq!((i[0].lower, i[0].upper), ('ı', 'I'));
        assert_eq!((i[1].lower, i[1].upper), ('i', 'İ'));
    }

    #[test]
    fn the_page_keys_lead_to_digits_symbols_and_back() {
        let mut keyboard = Keyboard::new(Target::Title, "", Language::En);
        keyboard.focus(3, 0);
        keyboard.press();
        assert_eq!(keyboard.page, Page::Digits);
        keyboard.focus(2, 0);
        keyboard.press();
        assert_eq!(keyboard.page, Page::Symbols);
        keyboard.press();
        assert_eq!(keyboard.page, Page::Digits);
        keyboard.focus(3, 0);
        keyboard.press();
        assert_eq!(keyboard.page, Page::Letters);
    }

    #[test]
    fn up_and_down_land_on_the_key_drawn_under() {
        let mut keyboard = Keyboard::new(Target::Title, "", Language::En);
        // From `p` (top right) down: `l`, then backspace, then Enter.
        keyboard.focus(0, 9);
        keyboard.step(0, 1);
        assert_eq!(keyboard.key().lower, 'l');
        keyboard.step(0, 1);
        assert_eq!(keyboard.key().action, Action::Delete);
        keyboard.step(0, 1);
        assert_eq!(keyboard.key().action, Action::Enter);
    }
}
