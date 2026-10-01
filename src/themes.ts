// Launcher themes.
//
// Every theme is a set of class strings, one per UI role. PocketJS compiles
// class literals at build time and does not accept interpolated fragments
// (`bg-[${color}]`), so each role spells out its full literal for every
// theme. Change a color by editing its literals here; add a theme by copying
// one block, giving it a new id, and adding the id to `ThemeId`.
//
// Roles are grouped by where they are used: screen and header, carousel
// card, grid tile, list row, footer, and the SELECT menu.

export type ThemeId = "midnight" | "aurora" | "sakura" | "ember" | "daylight";

export interface Theme {
  id: ThemeId;
  name: string;
  screen: string;
  title: string;
  crumbSep: string;
  crumb: string;
  pill: string;
  tab: string;
  tabActive: string;
  tabText: string;
  tabTextActive: string;
  key: string;
  keyActive: string;
  keyWide: string;
  keyWideActive: string;
  panelWide: string;
  pillText: string;
  card: string;
  cardTitle: string;
  cardMeta: string;
  cardId: string;
  tile: string;
  tileTitle: string;
  row: string;
  rowTitle: string;
  footerAccent: string;
  footerDim: string;
  empty: string;
  menuPanel: string;
  menuTitle: string;
  menuRow: string;
  menuRowActive: string;
  menuLabel: string;
  menuValue: string;
  menuHint: string;
}

export const DEFAULT_THEME: ThemeId = "midnight";

export const THEMES: readonly Theme[] = [
  {
    id: "midnight",
    name: "Midnight",
    screen: "w-full h-full relative flex-col justify-between py-1 bg-[#020617]",
    title: "text-base font-bold text-[#ffffff]",
    crumbSep: "text-xs text-[#64748b]",
    crumb: "text-xs text-[#94a3b8]",
    pill: "flex-row items-center px-2 py-1 rounded-md bg-[#1e293b] border border-[#334155]",
    pillText: "text-xs font-bold text-[#22d3ee]",
    tab: "flex-row items-center justify-center w-[96] py-1 rounded-md border border-[#00000000]",
    tabActive: "flex-row items-center justify-center w-[96] py-1 rounded-md bg-[#1e293b] border border-[#334155]",
    tabText: "text-xs text-[#94a3b8]",
    tabTextActive: "text-xs font-bold text-[#22d3ee]",
    key: "flex-row items-center justify-center w-[36] h-[26] rounded-md bg-[#1e293b] border border-[#334155]",
    keyActive: "flex-row items-center justify-center w-[36] h-[26] rounded-md bg-[#164e63] border border-[#22d3ee]",
    keyWide: "flex-row items-center justify-center w-[72] h-[26] rounded-md bg-[#1e293b] border border-[#334155]",
    keyWideActive: "flex-row items-center justify-center w-[72] h-[26] rounded-md bg-[#164e63] border border-[#22d3ee]",
    panelWide: "flex-col gap-1 w-[430] p-2 rounded-xl bg-[#0f172a] border border-[#475569]",
    card: "flex-col shrink-0 w-[150] h-[165] p-2 rounded-xl bg-[#0f172a] border border-[#334155] focus:border-[#22d3ee] focus:bg-[#1e293b] translate-y-1 focus:translate-y-0 transition-all duration-150 ease-out",
    cardTitle: "text-xs font-bold text-[#ffffff]",
    cardMeta: "text-xs text-[#94a3b8]",
    cardId: "text-xs font-bold text-[#22d3ee]",
    tile: "flex-col shrink-0 w-[104] h-[84] p-1 rounded-lg bg-[#0f172a] border border-[#334155] focus:border-[#22d3ee] focus:bg-[#1e293b] transition-all duration-150 ease-out",
    tileTitle: "mt-1 text-xs font-bold text-[#ffffff]",
    row: "flex-row shrink-0 items-center gap-3 w-full h-[34] px-2 rounded-lg bg-[#0f172a] border border-[#334155] focus:border-[#22d3ee] focus:bg-[#1e293b] transition-all duration-150 ease-out",
    rowTitle: "grow text-sm font-bold text-[#ffffff]",
    footerAccent: "text-xs font-bold text-[#22d3ee]",
    footerDim: "text-xs text-[#94a3b8]",
    empty: "text-sm text-[#94a3b8]",
    menuPanel: "flex-col gap-1 w-[340] p-3 rounded-xl bg-[#0f172a] border border-[#475569]",
    menuTitle: "text-base font-bold text-[#ffffff]",
    menuRow: "flex-row items-center justify-between h-[28] px-3 rounded-lg bg-[#1e293b] border border-[#334155]",
    menuRowActive: "flex-row items-center justify-between h-[28] px-3 rounded-lg bg-[#164e63] border border-[#22d3ee]",
    menuLabel: "text-sm font-bold text-[#ffffff]",
    menuValue: "text-sm font-bold text-[#22d3ee]",
    menuHint: "text-xs text-[#94a3b8]",
  },
  {
    id: "aurora",
    name: "Aurora",
    screen: "w-full h-full relative flex-col justify-between py-1 bg-[#04130f]",
    title: "text-base font-bold text-[#ecfdf5]",
    crumbSep: "text-xs text-[#5f8a7a]",
    crumb: "text-xs text-[#8fb8a8]",
    pill: "flex-row items-center px-2 py-1 rounded-md bg-[#0f2f26] border border-[#1f4d40]",
    pillText: "text-xs font-bold text-[#34d399]",
    tab: "flex-row items-center justify-center w-[96] py-1 rounded-md border border-[#00000000]",
    tabActive: "flex-row items-center justify-center w-[96] py-1 rounded-md bg-[#0f2f26] border border-[#1f4d40]",
    tabText: "text-xs text-[#8fb8a8]",
    tabTextActive: "text-xs font-bold text-[#34d399]",
    key: "flex-row items-center justify-center w-[36] h-[26] rounded-md bg-[#11382d] border border-[#1f4d40]",
    keyActive: "flex-row items-center justify-center w-[36] h-[26] rounded-md bg-[#155e46] border border-[#34d399]",
    keyWide: "flex-row items-center justify-center w-[72] h-[26] rounded-md bg-[#11382d] border border-[#1f4d40]",
    keyWideActive: "flex-row items-center justify-center w-[72] h-[26] rounded-md bg-[#155e46] border border-[#34d399]",
    panelWide: "flex-col gap-1 w-[430] p-2 rounded-xl bg-[#0b2a22] border border-[#2b6a57]",
    card: "flex-col shrink-0 w-[150] h-[165] p-2 rounded-xl bg-[#0b2a22] border border-[#1f4d40] focus:border-[#34d399] focus:bg-[#11382d] translate-y-1 focus:translate-y-0 transition-all duration-150 ease-out",
    cardTitle: "text-xs font-bold text-[#ecfdf5]",
    cardMeta: "text-xs text-[#8fb8a8]",
    cardId: "text-xs font-bold text-[#34d399]",
    tile: "flex-col shrink-0 w-[104] h-[84] p-1 rounded-lg bg-[#0b2a22] border border-[#1f4d40] focus:border-[#34d399] focus:bg-[#11382d] transition-all duration-150 ease-out",
    tileTitle: "mt-1 text-xs font-bold text-[#ecfdf5]",
    row: "flex-row shrink-0 items-center gap-3 w-full h-[34] px-2 rounded-lg bg-[#0b2a22] border border-[#1f4d40] focus:border-[#34d399] focus:bg-[#11382d] transition-all duration-150 ease-out",
    rowTitle: "grow text-sm font-bold text-[#ecfdf5]",
    footerAccent: "text-xs font-bold text-[#34d399]",
    footerDim: "text-xs text-[#8fb8a8]",
    empty: "text-sm text-[#8fb8a8]",
    menuPanel: "flex-col gap-1 w-[340] p-3 rounded-xl bg-[#0b2a22] border border-[#2b6a57]",
    menuTitle: "text-base font-bold text-[#ecfdf5]",
    menuRow: "flex-row items-center justify-between h-[28] px-3 rounded-lg bg-[#11382d] border border-[#1f4d40]",
    menuRowActive: "flex-row items-center justify-between h-[28] px-3 rounded-lg bg-[#155e46] border border-[#34d399]",
    menuLabel: "text-sm font-bold text-[#ecfdf5]",
    menuValue: "text-sm font-bold text-[#34d399]",
    menuHint: "text-xs text-[#8fb8a8]",
  },
  {
    id: "sakura",
    name: "Sakura",
    screen: "w-full h-full relative flex-col justify-between py-1 bg-[#1a0b16]",
    title: "text-base font-bold text-[#fff1f7]",
    crumbSep: "text-xs text-[#9c6f8a]",
    crumb: "text-xs text-[#cfa3bd]",
    pill: "flex-row items-center px-2 py-1 rounded-md bg-[#3a1a32] border border-[#55294b]",
    pillText: "text-xs font-bold text-[#f472b6]",
    tab: "flex-row items-center justify-center w-[96] py-1 rounded-md border border-[#00000000]",
    tabActive: "flex-row items-center justify-center w-[96] py-1 rounded-md bg-[#3a1a32] border border-[#55294b]",
    tabText: "text-xs text-[#cfa3bd]",
    tabTextActive: "text-xs font-bold text-[#f472b6]",
    key: "flex-row items-center justify-center w-[36] h-[26] rounded-md bg-[#3a1a32] border border-[#55294b]",
    keyActive: "flex-row items-center justify-center w-[36] h-[26] rounded-md bg-[#6b2357] border border-[#f472b6]",
    keyWide: "flex-row items-center justify-center w-[72] h-[26] rounded-md bg-[#3a1a32] border border-[#55294b]",
    keyWideActive: "flex-row items-center justify-center w-[72] h-[26] rounded-md bg-[#6b2357] border border-[#f472b6]",
    panelWide: "flex-col gap-1 w-[430] p-2 rounded-xl bg-[#2a1324] border border-[#7a3a69]",
    card: "flex-col shrink-0 w-[150] h-[165] p-2 rounded-xl bg-[#2a1324] border border-[#55294b] focus:border-[#f472b6] focus:bg-[#3a1a32] translate-y-1 focus:translate-y-0 transition-all duration-150 ease-out",
    cardTitle: "text-xs font-bold text-[#fff1f7]",
    cardMeta: "text-xs text-[#cfa3bd]",
    cardId: "text-xs font-bold text-[#f472b6]",
    tile: "flex-col shrink-0 w-[104] h-[84] p-1 rounded-lg bg-[#2a1324] border border-[#55294b] focus:border-[#f472b6] focus:bg-[#3a1a32] transition-all duration-150 ease-out",
    tileTitle: "mt-1 text-xs font-bold text-[#fff1f7]",
    row: "flex-row shrink-0 items-center gap-3 w-full h-[34] px-2 rounded-lg bg-[#2a1324] border border-[#55294b] focus:border-[#f472b6] focus:bg-[#3a1a32] transition-all duration-150 ease-out",
    rowTitle: "grow text-sm font-bold text-[#fff1f7]",
    footerAccent: "text-xs font-bold text-[#f472b6]",
    footerDim: "text-xs text-[#cfa3bd]",
    empty: "text-sm text-[#cfa3bd]",
    menuPanel: "flex-col gap-1 w-[340] p-3 rounded-xl bg-[#2a1324] border border-[#7a3a69]",
    menuTitle: "text-base font-bold text-[#fff1f7]",
    menuRow: "flex-row items-center justify-between h-[28] px-3 rounded-lg bg-[#3a1a32] border border-[#55294b]",
    menuRowActive: "flex-row items-center justify-between h-[28] px-3 rounded-lg bg-[#6b2357] border border-[#f472b6]",
    menuLabel: "text-sm font-bold text-[#fff1f7]",
    menuValue: "text-sm font-bold text-[#f472b6]",
    menuHint: "text-xs text-[#cfa3bd]",
  },
  {
    id: "ember",
    name: "Ember",
    screen: "w-full h-full relative flex-col justify-between py-1 bg-[#120d0a]",
    title: "text-base font-bold text-[#fff7ed]",
    crumbSep: "text-xs text-[#8c7463]",
    crumb: "text-xs text-[#c2a48f]",
    pill: "flex-row items-center px-2 py-1 rounded-md bg-[#2e2119] border border-[#46352a]",
    pillText: "text-xs font-bold text-[#fb923c]",
    tab: "flex-row items-center justify-center w-[96] py-1 rounded-md border border-[#00000000]",
    tabActive: "flex-row items-center justify-center w-[96] py-1 rounded-md bg-[#2e2119] border border-[#46352a]",
    tabText: "text-xs text-[#c2a48f]",
    tabTextActive: "text-xs font-bold text-[#fb923c]",
    key: "flex-row items-center justify-center w-[36] h-[26] rounded-md bg-[#2e2119] border border-[#46352a]",
    keyActive: "flex-row items-center justify-center w-[36] h-[26] rounded-md bg-[#7c3a12] border border-[#fb923c]",
    keyWide: "flex-row items-center justify-center w-[72] h-[26] rounded-md bg-[#2e2119] border border-[#46352a]",
    keyWideActive: "flex-row items-center justify-center w-[72] h-[26] rounded-md bg-[#7c3a12] border border-[#fb923c]",
    panelWide: "flex-col gap-1 w-[430] p-2 rounded-xl bg-[#211813] border border-[#6b4a36]",
    card: "flex-col shrink-0 w-[150] h-[165] p-2 rounded-xl bg-[#211813] border border-[#46352a] focus:border-[#fb923c] focus:bg-[#2e2119] translate-y-1 focus:translate-y-0 transition-all duration-150 ease-out",
    cardTitle: "text-xs font-bold text-[#fff7ed]",
    cardMeta: "text-xs text-[#c2a48f]",
    cardId: "text-xs font-bold text-[#fb923c]",
    tile: "flex-col shrink-0 w-[104] h-[84] p-1 rounded-lg bg-[#211813] border border-[#46352a] focus:border-[#fb923c] focus:bg-[#2e2119] transition-all duration-150 ease-out",
    tileTitle: "mt-1 text-xs font-bold text-[#fff7ed]",
    row: "flex-row shrink-0 items-center gap-3 w-full h-[34] px-2 rounded-lg bg-[#211813] border border-[#46352a] focus:border-[#fb923c] focus:bg-[#2e2119] transition-all duration-150 ease-out",
    rowTitle: "grow text-sm font-bold text-[#fff7ed]",
    footerAccent: "text-xs font-bold text-[#fb923c]",
    footerDim: "text-xs text-[#c2a48f]",
    empty: "text-sm text-[#c2a48f]",
    menuPanel: "flex-col gap-1 w-[340] p-3 rounded-xl bg-[#211813] border border-[#6b4a36]",
    menuTitle: "text-base font-bold text-[#fff7ed]",
    menuRow: "flex-row items-center justify-between h-[28] px-3 rounded-lg bg-[#2e2119] border border-[#46352a]",
    menuRowActive: "flex-row items-center justify-between h-[28] px-3 rounded-lg bg-[#7c3a12] border border-[#fb923c]",
    menuLabel: "text-sm font-bold text-[#fff7ed]",
    menuValue: "text-sm font-bold text-[#fb923c]",
    menuHint: "text-xs text-[#c2a48f]",
  },
  {
    id: "daylight",
    name: "Daylight",
    screen: "w-full h-full relative flex-col justify-between py-1 bg-[#eef2f7]",
    title: "text-base font-bold text-[#0f172a]",
    crumbSep: "text-xs text-[#94a3b8]",
    crumb: "text-xs text-[#475569]",
    pill: "flex-row items-center px-2 py-1 rounded-md bg-[#ffffff] border border-[#cbd5e1]",
    pillText: "text-xs font-bold text-[#4f46e5]",
    tab: "flex-row items-center justify-center w-[96] py-1 rounded-md border border-[#00000000]",
    tabActive: "flex-row items-center justify-center w-[96] py-1 rounded-md bg-[#ffffff] border border-[#cbd5e1]",
    tabText: "text-xs text-[#475569]",
    tabTextActive: "text-xs font-bold text-[#4f46e5]",
    key: "flex-row items-center justify-center w-[36] h-[26] rounded-md bg-[#f1f5f9] border border-[#cbd5e1]",
    keyActive: "flex-row items-center justify-center w-[36] h-[26] rounded-md bg-[#e0e7ff] border border-[#4f46e5]",
    keyWide: "flex-row items-center justify-center w-[72] h-[26] rounded-md bg-[#f1f5f9] border border-[#cbd5e1]",
    keyWideActive: "flex-row items-center justify-center w-[72] h-[26] rounded-md bg-[#e0e7ff] border border-[#4f46e5]",
    panelWide: "flex-col gap-1 w-[430] p-2 rounded-xl bg-[#ffffff] border border-[#94a3b8]",
    card: "flex-col shrink-0 w-[150] h-[165] p-2 rounded-xl bg-[#ffffff] border border-[#cbd5e1] focus:border-[#4f46e5] focus:bg-[#e0e7ff] translate-y-1 focus:translate-y-0 transition-all duration-150 ease-out",
    cardTitle: "text-xs font-bold text-[#0f172a]",
    cardMeta: "text-xs text-[#475569]",
    cardId: "text-xs font-bold text-[#4f46e5]",
    tile: "flex-col shrink-0 w-[104] h-[84] p-1 rounded-lg bg-[#ffffff] border border-[#cbd5e1] focus:border-[#4f46e5] focus:bg-[#e0e7ff] transition-all duration-150 ease-out",
    tileTitle: "mt-1 text-xs font-bold text-[#0f172a]",
    row: "flex-row shrink-0 items-center gap-3 w-full h-[34] px-2 rounded-lg bg-[#ffffff] border border-[#cbd5e1] focus:border-[#4f46e5] focus:bg-[#e0e7ff] transition-all duration-150 ease-out",
    rowTitle: "grow text-sm font-bold text-[#0f172a]",
    footerAccent: "text-xs font-bold text-[#4f46e5]",
    footerDim: "text-xs text-[#475569]",
    empty: "text-sm text-[#475569]",
    menuPanel: "flex-col gap-1 w-[340] p-3 rounded-xl bg-[#ffffff] border border-[#94a3b8]",
    menuTitle: "text-base font-bold text-[#0f172a]",
    menuRow: "flex-row items-center justify-between h-[28] px-3 rounded-lg bg-[#f1f5f9] border border-[#cbd5e1]",
    menuRowActive: "flex-row items-center justify-between h-[28] px-3 rounded-lg bg-[#e0e7ff] border border-[#4f46e5]",
    menuLabel: "text-sm font-bold text-[#0f172a]",
    menuValue: "text-sm font-bold text-[#4f46e5]",
    menuHint: "text-xs text-[#475569]",
  },
];

export function themeById(id: ThemeId): Theme {
  return THEMES.find((theme) => theme.id === id) ?? THEMES[0];
}
