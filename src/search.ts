// Title search (square button): which titles a typed term finds, and in what order.

// Letters that are not a base letter plus a mark, with what they match.
const PLAIN: Record<string, string> = { ı: "i", ß: "ss", æ: "ae", œ: "oe" };

/**
 * Lowercase and without the marks on letters, so "pokemon" finds "Pokémon"
 * and "isik" finds "Işık".
 */
export function searchKey(text: string): string {
  // `normalize` is missing from a JS engine built without Unicode tables.
  const plain = typeof text.normalize === "function" ? text.normalize("NFD").replace(/[\u0300-\u036f]/g, "") : text;
  return plain.toLowerCase().replace(/[ıßæœ]/g, (letter) => PLAIN[letter]);
}

/**
 * The titles `term` finds. Every word of the term has to be part of the title
 * or of the title id. Titles that start with the first word come first, then
 * titles with a word that starts with it, then the rest; inside each group
 * the order of `games` is kept. An empty term finds nothing.
 */
export function searchTitles<T extends { title: string; id: string }>(games: readonly T[], term: string): T[] {
  const words = searchKey(term).split(/\s+/).filter(Boolean);
  if (words.length === 0) return [];
  const found: { game: T; rank: number; index: number }[] = [];
  for (const [index, game] of games.entries()) {
    const title = searchKey(game.title);
    const id = game.id.toLowerCase();
    if (!words.every((word) => title.includes(word) || id.includes(word))) continue;
    const first = words[0];
    const rank = title.startsWith(first)
      ? 0
      : title.split(/[^a-z0-9]+/).some((part) => part.startsWith(first))
        ? 1
        : 2;
    found.push({ game, rank, index });
  }
  return found.sort((a, b) => a.rank - b.rank || a.index - b.index).map(({ game }) => game);
}
