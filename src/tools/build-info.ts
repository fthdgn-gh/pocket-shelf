// Writes src/build-info.json: which build this is, so the app can tell
// whether a release on GitHub is newer (src/updates.ts). Run before every
// shelf build, check and test; the file is not committed.
//
//   version  the release's version when built from a `shelf-v<version>` tag
//            (CI sets GITHUB_REF_NAME), else pocket.json's version
//   channel  "stable" or "beta" from the tag, "nightly" for any other CI
//            build, "local" on a developer's machine
//   commit   the full commit id the build was made from, "" when unknown

import { $ } from "bun";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const manifest = (await Bun.file(join(ROOT, "pocket.json")).json()) as { version: string };

const ref = process.env.GITHUB_REF_NAME ?? "";
const tag = /^shelf-v(\d+\.\d+\.\d+(?:-beta\.\d+)?)$/.exec(ref);
const version = tag ? tag[1]! : manifest.version;
const channel = tag ? (version.includes("-beta.") ? "beta" : "stable") : process.env.GITHUB_ACTIONS ? "nightly" : "local";
const commit =
  process.env.GITHUB_SHA ?? (await $`git rev-parse HEAD`.cwd(ROOT).quiet().nothrow().text()).trim();

const info = { version, channel, commit: /^[0-9a-f]{40}$/.test(commit) ? commit : "" };
await Bun.write(join(ROOT, "src/build-info.json"), `${JSON.stringify(info, null, 2)}\n`);
console.log(`Pocket Shelf build ${info.version} (${info.channel}${info.commit ? `, ${info.commit.slice(0, 7)}` : ""})`);
