// Languages of the launcher's own text. Titles come from each title's files
// and are not translated.
//
// Add a language by copying a file in locales/, listing it here, and adding
// the characters it needs to the ranges in fonts.json: a character the font
// was not baked with draws as an empty box.

import { de } from "./locales/de.ts";
import { en, type Messages } from "./locales/en.ts";
import { es } from "./locales/es.ts";
import { fr } from "./locales/fr.ts";
import { tr } from "./locales/tr.ts";

export type { Messages };

export type Language = "en" | "tr" | "de" | "fr" | "es";

/** Each language under its own name, in menu order. */
export const LANGUAGES: readonly { id: Language; name: string }[] = [
  { id: "en", name: "English" },
  { id: "tr", name: "Türkçe" },
  { id: "de", name: "Deutsch" },
  { id: "fr", name: "Français" },
  { id: "es", name: "Español" },
];

export const DEFAULT_LANGUAGE: Language = "en";

export const MESSAGES: Record<Language, Messages> = { en, tr, de, fr, es };

/** `template` with each `{name}` replaced by its value. */
export function fill(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (whole, name: string) => (name in values ? String(values[name]) : whole));
}

/**
 * Upper case for a heading. Turkish has a dotted and a dotless i, each with
 * its own capital, which the engine's `toUpperCase` does not know.
 */
export function upper(text: string, language: Language): string {
  const prepared = language === "tr" ? text.replace(/i/g, "İ").replace(/ı/g, "I") : text;
  return prepared.toUpperCase();
}
