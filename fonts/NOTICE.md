# Fonts

Figtree is baked into Pocket Shelf by `bun run shelf:build`; it fills the
engine's sans slots. `src/tools/icons.ts` and `src/tools/art.ts` draw the
letters in the button icons and the LiveArea pictures with Figtree Bold.
Inter (assets/fonts) supplies symbols Figtree does not have.

- Figtree (Regular, Bold): SIL Open Font License 1.1, `OFL-Figtree.txt`,
  https://github.com/erikdkennedy/figtree

Keep each font's license file next to it when you add one.
