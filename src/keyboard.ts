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

/** Column in a row of `to` keys that sits under column `col` of a row of `from`. */
export function mapColumn(from: number, to: number, col: number): number {
  return Math.min(to - 1, Math.floor(((col + 0.5) * to) / from));
}
