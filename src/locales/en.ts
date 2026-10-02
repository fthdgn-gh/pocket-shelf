// English, and the shape every other language follows. A `{name}` in a text
// is replaced when it is shown (see `fill` in i18n.ts); `{button}` is where a
// button icon goes.

export const en = {
  // Button hints
  launch: "Launch",
  edit: "Edit",
  search: "Search",
  close: "Close",
  menu: "Menu",
  back: "Back",
  change: "Change",
  /** Move the highlight. */
  navigate: "Move",
  /** Move a category earlier or later. */
  reorder: "Move",
  use: "Use",
  next: "Next",
  choose: "Choose",
  type: "Type",
  rename: "Rename",
  create: "New",
  remove: "Delete",
  hide: "Hide",
  show: "Show",
  key: "Key",
  shift: "Shift",
  symbols: "Symbols",
  done: "Done",
  cancel: "Cancel",

  // Values
  on: "On",
  off: "Off",
  none: "None",
  standard: "Default",

  // Main screen
  launching: "Launching {title}...",
  emptyCategory: "Nothing in this category yet",
  moveHere: "Press {button} on a title to move it here",
  noTitles: "No installed apps found",
  noMatch: 'No title matches "{term}"',
  searchAgain: "Search again",
  found: "{count} found",
  categories: {
    recent: "Last Played",
    favorites: "Favorites",
    search: "Search",
    games: "Games",
    system: "System",
    homebrew: "Homebrew",
  },

  // SELECT menu
  theme: "Theme",
  font: "Font",
  view: "View",
  details: "Details",
  backdrop: "Backdrop",
  iconBox: "Icon box",
  categoriesRow: "Categories",
  manage: "Manage",
  confirm: "Confirm",
  language: "Language",
  views: { carousel: "Carousel", grid: "Grid", list: "List" },
  detailLevels: { basic: "Basic", normal: "Normal", detailed: "Detailed" },
  themes: {
    midnight: "Midnight",
    aurora: "Aurora",
    sakura: "Sakura",
    ember: "Ember",
    daylight: "Daylight",
    dynamic: "Dynamic",
  },

  // Title editor
  editorTitle: "Edit title",
  favorite: "Favorite",
  category: "Category",
  title: "Title",
  boxArt: "Box art",
  reset: "Reset to defaults",
  resetDone: "Reset to the defaults",
  gameIcon: "Game icon",
  autoArt: "{file} (auto)",
  noPng: "No PNG files yet. Copy images to",

  // Category manager
  newCategory: "New category",
  hidden: "Hidden",
  builtinKeepNames: "Built-in categories keep their names.",
  builtinNoDelete: "Built-in categories can't be deleted. Hide them instead.",
  nameTaken: "There is already a category with that name.",
  deleted: "Deleted",
  deleteAgain: "Press {button} again to delete {name}",
  nowShown: "{name} is shown",
  nowHidden: "{name} is hidden",
  oneVisible: "At least one category has to stay visible.",

  // Keyboard keys
  keySpace: "Space",
  keyDelete: "Delete",
  keyLetters: "Abc",

  // SteamGridDB
  enterKey: "Enter API key",
  /** Shown above the path of the key file. */
  keyHelp:
    "SteamGridDB needs your own API key. Create one at steamgriddb.com, under Preferences, API. Type it here, or save it as one line in the file",
  icon: "Icon",
  searching: "Searching...",
  loadingArt: "Loading artwork...",
  loading: "Loading...",
  downloading: "Downloading...",
  downloadingPercent: "Downloading... {percent}%",
  downloadingSize: "Downloading... {size} KB",
  unreachable: "Could not reach SteamGridDB ({error}).",
  noGames: "No games found. Change the search.",
  noIcons: "No icons for this game.",
  noBackdrops: "No backdrops for this game.",
  unreadable: "That image could not be read.",
  downloadFailed: "Download failed ({error}).",
  statusCode: "status {status}",
  noNetwork: "This device has no network support.",
  iconApplied: "Icon applied.",
  backdropApplied: "Backdrop applied.",
  badKey: "That does not look like an API key.",
  keyRejected: "SteamGridDB did not accept the API key.",
  serverBusy: "SteamGridDB is busy. Try again in a moment.",
  serverStatus: "SteamGridDB answered with status {status}.",
  badReply: "SteamGridDB sent an unexpected reply.",
};

export type Messages = typeof en;
