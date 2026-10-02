/** One key of the on-screen keyboard. */
export interface KeyDef {
  action: "char" | "shift" | "symbols" | "space" | "delete" | "done";
  lower: string;
  upper: string;
  wide?: boolean;
}

const chars = (lower: string, upper = lower.toUpperCase()): KeyDef[] =>
  [...lower].map((character, index) => ({ action: "char", lower: character, upper: upper[index] }));

const BOTTOM_ROW: readonly KeyDef[] = [
  { action: "shift", lower: "Shift", upper: "Shift", wide: true },
  { action: "symbols", lower: "Symbols", upper: "Symbols", wide: true },
  { action: "space", lower: "Space", upper: "Space", wide: true },
  { action: "delete", lower: "Delete", upper: "Delete", wide: true },
  { action: "done", lower: "Done", upper: "Done", wide: true },
];

export const LETTER_ROWS: readonly (readonly KeyDef[])[] = [
  chars("1234567890", "!@#$%&*()?"),
  chars("qwertyuiop"),
  chars("asdfghjkl-", "ASDFGHJKL_"),
  chars("zxcvbnm.,'", "ZXCVBNM:;\""),
  BOTTOM_ROW,
];

export const SYMBOL_ROWS: readonly (readonly KeyDef[])[] = [
  chars("!@#$%^&*()", "!@#$%^&*()"),
  chars("-_=+[]{}<>", "-_=+[]{}<>"),
  chars(";:'\",./?\\|", ";:'\",./?\\|"),
  chars("~`", "~`"),
  BOTTOM_ROW,
];

/** The keyboard's pages, in the order the page key steps through them. */
export type KeyPage = "letters" | "symbols" | "accents";
export const KEY_PAGES: readonly KeyPage[] = ["letters", "symbols", "accents"];

// Letters with marks and the other letters the four translated languages
// use, as [lower, upper] in the order they are laid out. `i` is here for its
// Turkish capital; `ß`, `¿` and `¡` have no capital in the baked font.
const ACCENTS: readonly (readonly [string, string])[] = [
  ["á", "Á"], ["à", "À"], ["â", "Â"], ["ä", "Ä"], ["æ", "Æ"], ["ç", "Ç"], ["é", "É"], ["è", "È"], ["ê", "Ê"], ["ë", "Ë"],
  ["í", "Í"], ["î", "Î"], ["ï", "Ï"], ["ı", "I"], ["i", "İ"], ["ñ", "Ñ"], ["ó", "Ó"], ["ô", "Ô"], ["ö", "Ö"], ["œ", "Œ"],
  ["ú", "Ú"], ["ù", "Ù"], ["û", "Û"], ["ü", "Ü"], ["ÿ", "Ÿ"], ["ğ", "Ğ"], ["ş", "Ş"], ["ß", "ß"], ["¿", "¿"], ["¡", "¡"],
];

/** The letters each language types most, which lead its accents page. */
export const ACCENTS_FIRST: Record<string, string> = {
  tr: "çğıiöşü",
  de: "äöüß",
  fr: "éèêëàâçîïôùûüÿœæ",
  es: "áéíóúñü¿¡",
};

/**
 * The accents page for a language: the same thirty keys in every language,
 * with that language's own letters first. Three rows of ten under the digits.
 */
export function accentRows(language: string): readonly (readonly KeyDef[])[] {
  const first = [...(ACCENTS_FIRST[language] ?? "")];
  const rank = (lower: string) => {
    const index = first.indexOf(lower);
    return index < 0 ? first.length : index;
  };
  const keys: KeyDef[] = ACCENTS.map(([lower, upper], index) => ({ lower, upper, index }))
    .sort((a, b) => rank(a.lower) - rank(b.lower) || a.index - b.index)
    .map(({ lower, upper }) => ({ action: "char", lower, upper }));
  return [LETTER_ROWS[0], keys.slice(0, 10), keys.slice(10, 20), keys.slice(20, 30), BOTTOM_ROW];
}

/** Column in a row of `to` keys that sits under column `col` of a row of `from`. */
export function mapColumn(from: number, to: number, col: number): number {
  return Math.min(to - 1, Math.floor(((col + 0.5) * to) / from));
}
