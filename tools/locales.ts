// Writes core/src/locales.rs from locales/*.ts: one `Messages` struct in the
// shape of `en`, and a constant per language with the same strings. The
// TypeScript files are the source; run `bun run locales` after a text
// changes there.

import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { de } from "../locales/de.ts";
import { en } from "../locales/en.ts";
import { es } from "../locales/es.ts";
import { fr } from "../locales/fr.ts";
import { tr } from "../locales/tr.ts";

type Tree = { [key: string]: string | Tree };

// Rust keywords among the keys (`use`, `type`) become raw identifiers.
const KEYWORDS = new Set(["use", "type", "move", "ref", "box", "match", "loop", "where", "static", "self", "crate", "mod", "impl", "fn", "let", "in", "as"]);
const snake = (key: string) => {
  const name = key.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);
  // The clock formats "24" and "12" are `h24` and `h12`.
  if (/^\d/.test(name)) return `h${name}`;
  return KEYWORDS.has(name) ? `r#${name}` : name;
};
const pascal = (key: string) => key[0].toUpperCase() + key.slice(1);
// A JSON string is a Rust string literal except for `\uXXXX`.
const literal = (text: string) => JSON.stringify(text).replace(/\\u([0-9a-fA-F]{4})/g, "\\u{$1}");

const structs: string[] = [];

function struct(name: string, shape: Tree) {
  const fields = Object.entries(shape).map(([key, value]) =>
    typeof value === "string" ? `    pub ${snake(key)}: &'static str,` : `    pub ${snake(key)}: ${pascal(key)},`,
  );
  for (const [key, value] of Object.entries(shape)) if (typeof value !== "string") struct(pascal(key), value);
  structs.unshift(`#[derive(Debug)]\npub struct ${name} {\n${fields.join("\n")}\n}\n`);
}

function value(name: string, shape: Tree, texts: Tree, indent: string): string {
  const lines = Object.entries(shape).map(([key, model]) => {
    const text = texts[key];
    if (text === undefined) throw new Error(`missing ${key} in a language`);
    return typeof model === "string"
      ? `${indent}    ${snake(key)}: ${literal(text as string)},`
      : `${indent}    ${snake(key)}: ${value(pascal(key), model, text as Tree, `${indent}    `)},`;
  });
  return `${name} {\n${lines.join("\n")}\n${indent}}`;
}

struct("Messages", en as unknown as Tree);

// Every text with its key, for the tests that compare languages.
const paths: string[] = [];
const walk = (shape: Tree, prefix: string, access: string) => {
  for (const [key, model] of Object.entries(shape)) {
    if (typeof model === "string") paths.push(`            (${literal(prefix + key)}, self.${access}${snake(key)}),`);
    else walk(model, `${prefix}${key}.`, `${access}${snake(key)}.`);
  }
};
walk(en as unknown as Tree, "", "");
const texts = `impl Messages {
    /// Every text with its key (\`categories.games\`), in \`en.ts\` order.
    pub fn texts(&self) -> Vec<(&'static str, &'static str)> {
        vec![
${paths.join("\n")}
        ]
    }
}
`;
const languages: [string, Tree][] = [
  ["EN", en as unknown as Tree],
  ["TR", tr as unknown as Tree],
  ["DE", de as unknown as Tree],
  ["FR", fr as unknown as Tree],
  ["ES", es as unknown as Tree],
];

const out = [
  "//! The launcher's texts in its five languages, generated from",
  "//! `locales/*.ts` by `tools/locales.ts` (`bun run locales`). Do not edit",
  "//! by hand. A `{name}` in a text",
  "//! is filled in when shown (`i18n::fill`); `{button}` is where a button's",
  "//! symbol goes.",
  "",
  ...structs,
  texts,
  ...languages.map(([name, texts]) => `pub static ${name}: Messages = ${value("Messages", en as unknown as Tree, texts, "")};\n`),
].join("\n");

writeFileSync(join(import.meta.dir, "../core/src/locales.rs"), out);
console.log(`locales.rs: ${languages.length} languages, ${structs.length} structs`);
