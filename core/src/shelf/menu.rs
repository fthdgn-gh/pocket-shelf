//! The SELECT menu's pages and rows. A row named after a page opens that
//! page: the main page holds Appearance (how the shelf looks), Games (what
//! is listed and what is known about it) and System (the console and the
//! app); Appearance holds Status bar, Games holds Artwork. No page has more
//! rows than show at once (`VISIBLE`). Language is the main page's last row,
//! one press up from the first, so it can be found in a language the user
//! cannot read.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Main,
    Appearance,
    Status,
    Games,
    Artwork,
    System,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
    Open(Page),
    /// The current tab's order.
    Sort,
    Theme,
    View,
    Details,
    TextSize,
    Backdrop,
    IconBox,
    StatusBar,
    StatusPosition,
    ShowWifi,
    ShowBluetooth,
    ShowBattery,
    Clock,
    BatteryPercent,
    Categories,
    /// The app that starts games both RetroArch and Emu4Vita++ open.
    Emulator,
    FetchArt,
    CleanArt,
    Rescan,
    /// SELECT ▸ Games ▸ Game database: download and identify.
    GameDatabase,
    /// The names identified titles show.
    GameNames,
    /// The details panel next to the selected title.
    GameDetails,
    UpdateChannel,
    CheckUpdates,
    Confirm,
    Language,
    /// The system's keyboard or Pocket Shelf's.
    Keyboard,
    Diagnostics,
    /// Open at console start and Return after a game (`autostart.rs`).
    ConsoleStart,
    /// Read the folder categories' folders again: on the main page, after
    /// Sort, while a folder category's tab is open.
    RefreshFolder,
}

pub const PAGES: [Page; 6] = [Page::Main, Page::Appearance, Page::Status, Page::Games, Page::Artwork, Page::System];

/// Rows shown at once; the menu scrolls with the highlight.
pub const VISIBLE: usize = 7;

pub fn items(page: Page) -> &'static [Item] {
    use Item::*;
    match page {
        Page::Main => &[Sort, Open(Page::Appearance), Open(Page::Games), Open(Page::System), Language],
        Page::Appearance => &[Theme, View, TextSize, Details, Backdrop, IconBox, Open(Page::Status)],
        Page::Status => &[StatusBar, StatusPosition, ShowWifi, ShowBluetooth, ShowBattery, BatteryPercent, Clock],
        Page::Games => &[Categories, Emulator, Rescan, GameDatabase, GameNames, GameDetails, Open(Page::Artwork)],
        Page::Artwork => &[FetchArt, CleanArt],
        Page::System => &[Confirm, Keyboard, ConsoleStart, UpdateChannel, CheckUpdates, Diagnostics],
    }
}

/// The page holding the row that opens `page`; none for the main page.
pub fn parent_page(page: Page) -> Option<Page> {
    PAGES.into_iter().find(|p| items(*p).contains(&Item::Open(page)))
}

/// The pages to open, from the main page down, to reach the page holding
/// `item`.
pub fn menu_path(item: Item) -> Vec<Page> {
    let Some(holder) = PAGES.into_iter().find(|p| items(*p).contains(&item)) else { return Vec::new() };
    let mut path = vec![holder];
    let mut page = parent_page(holder);
    while let Some(p) = page {
        path.insert(0, p);
        page = parent_page(p);
    }
    path
}

/// The menu's place: its page and highlighted row. Kept while closed: it
/// opens again on the main page, on the group it was left in.
#[derive(Clone, Copy, Debug)]
pub struct Menu {
    pub open: bool,
    pub page: Page,
    pub row: usize,
    /// The open tab is a folder category's: the main page has Refresh
    /// folder.
    pub folder_tab: bool,
}

impl Default for Menu {
    fn default() -> Self {
        Menu { open: false, page: Page::Main, row: 0, folder_tab: false }
    }
}

impl Menu {
    /// The rows of the page shown: [`items`], with Refresh folder after
    /// Sort on a folder category's tab.
    pub fn rows(&self) -> Vec<Item> {
        let mut rows = items(self.page).to_vec();
        if self.page == Page::Main && self.folder_tab {
            rows.insert(1, Item::RefreshFolder);
        }
        rows
    }

    /// Show `page`, with the row that opens `from` highlighted, or its
    /// first row.
    pub fn show(&mut self, page: Page, from: Option<Page>) {
        self.page = page;
        self.row = from.and_then(|f| self.rows().iter().position(|i| *i == Item::Open(f))).unwrap_or(0);
    }

    /// Open on the main page: from a group, on the group's row there; from
    /// the main page, on the row it was left at. `folder_tab`: the open tab
    /// is a folder category's.
    pub fn open(&mut self, folder_tab: bool) {
        // The rows below Sort move with Refresh folder; the same row stays
        // highlighted.
        if self.page == Page::Main && self.folder_tab != folder_tab && self.row > 0 {
            self.row = if folder_tab { self.row + 1 } else { (self.row - 1).max(1) };
        }
        self.folder_tab = folder_tab;
        if self.page != Page::Main {
            let path = menu_path(Item::Open(self.page));
            let top = path.get(1).copied().unwrap_or(self.page);
            self.show(Page::Main, Some(top));
        }
        self.open = true;
    }

    /// Back to the page holding this one, on its row; the main page closes
    /// the menu.
    pub fn back(&mut self) {
        match parent_page(self.page) {
            Some(parent) => self.show(parent, Some(self.page)),
            None => self.open = false,
        }
    }

    pub fn item(&self) -> Item {
        let rows = self.rows();
        rows[self.row.min(rows.len() - 1)]
    }

    pub fn step_row(&mut self, delta: i32) {
        let count = self.rows().len() as i32;
        self.row = (self.row as i32 + delta).rem_euclid(count) as usize;
    }

    /// The first row shown: the highlight in the middle where it can be.
    pub fn first_visible(&self) -> usize {
        let count = self.rows().len();
        (self.row.saturating_sub(VISIBLE / 2)).min(count.saturating_sub(VISIBLE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_is_on_one_page_and_every_page_is_reached() {
        let rows: Vec<Item> = PAGES.iter().flat_map(|p| items(*p).iter().copied()).collect();
        for (i, row) in rows.iter().enumerate() {
            assert!(!rows[i + 1..].contains(row), "{row:?} twice");
        }
        for page in PAGES.into_iter().filter(|p| *p != Page::Main) {
            assert_eq!(menu_path(Item::Open(page))[0], Page::Main);
        }
    }

    #[test]
    fn status_bar_sits_inside_appearance() {
        assert_eq!(parent_page(Page::Status), Some(Page::Appearance));
        assert_eq!(menu_path(Item::Clock), [Page::Main, Page::Appearance, Page::Status]);
    }

    #[test]
    fn language_is_one_press_up_from_the_first_row() {
        let main = items(Page::Main);
        assert_eq!(main[main.len() - 1], Item::Language);
    }

    #[test]
    fn no_page_scrolls() {
        for page in PAGES {
            assert!(items(page).len() <= VISIBLE, "{page:?}: {} rows", items(page).len());
        }
        assert_eq!(menu_path(Item::CleanArt), [Page::Main, Page::Games, Page::Artwork]);
    }

    #[test]
    fn reopening_from_a_group_lands_on_its_row_in_the_main_page() {
        let mut menu = Menu::default();
        menu.open(false);
        menu.show(Page::Appearance, None);
        menu.show(Page::Status, None);
        menu.open = false;
        menu.open(false);
        assert_eq!((menu.page, menu.item()), (Page::Main, Item::Open(Page::Appearance)));
        // Back from a group returns to its row; from the main page it closes.
        menu.show(Page::Appearance, None);
        menu.show(Page::Status, None);
        menu.back();
        assert_eq!((menu.page, menu.item()), (Page::Appearance, Item::Open(Page::Status)));
        menu.back();
        menu.back();
        assert!(!menu.open);
    }
}
