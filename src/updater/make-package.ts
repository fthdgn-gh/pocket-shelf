// Pocket Shelf Updater (GPL-3.0, see LICENSE): builds the helper and lays out
// its installable package in src/vita/updater/, which Pocket Shelf's VPK
// carries as app0:updater/. Pocket Shelf installs it from there with the
// system's package installer, which needs sce_sys/package/head.bin; this
// script writes it as main.c's make_head_bin does on the console (the
// template and the fold of SHA-1 digests are VitaShell's).
//
//   bun run shelf:updater      (needs VitaSDK: VITASDK or ~/vitasdk)

import { $ } from "bun";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = resolve(fileURLToPath(new URL(".", import.meta.url)));
const ROOT = resolve(HERE, "../..");
const BUILD = join(ROOT, ".pocket-build/updater");
const OUT = join(ROOT, "src/vita/updater");
const TITLE_ID = "POCKTUPDR";

const vitasdk = process.env.VITASDK ?? join(homedir(), "vitasdk");
await $`make -C ${HERE}`.env({ ...process.env, VITASDK: vitasdk, PATH: `${vitasdk}/bin:${process.env.PATH}` });

function fpkgHmac(data: Uint8Array): Buffer {
  const sha1 = createHash("sha1").update(data).digest();
  const buf = Buffer.alloc(64);
  sha1.copy(buf, 0, 4, 12);
  sha1.copy(buf, 8, 4, 12);
  sha1.copy(buf, 16, 12, 16);
  buf[20] = sha1[16]!;
  buf[21] = sha1[1]!;
  buf[22] = sha1[2]!;
  buf[23] = sha1[3]!;
  buf.copy(buf, 24, 16, 24);
  return createHash("sha1").update(buf).digest().subarray(0, 16);
}

function headBin(titleId: string): Buffer {
  const head = Buffer.from(readFileSync(join(HERE, "head.bin")));
  head.fill(0, 0x30, 0x30 + 48);
  head.write(`EP9000-${titleId}_00-0000000000000000`, 0x30, "ascii");
  let length = head.readUInt32BE(0xd0);
  fpkgHmac(head.subarray(0, length)).copy(head, length);
  const offset = head.readUInt32BE(0x8);
  length = head.readUInt32BE(0x10);
  const out = head.readUInt32BE(0xd4);
  fpkgHmac(head.subarray(offset, offset + length - 64)).copy(head, out);
  length = head.readUInt32BE(0xe8);
  fpkgHmac(head.subarray(0, length)).copy(head, length);
  return head;
}

rmSync(OUT, { recursive: true, force: true });
mkdirSync(join(OUT, "sce_sys/package"), { recursive: true });
copyFileSync(join(BUILD, "eboot.bin"), join(OUT, "eboot.bin"));
copyFileSync(join(BUILD, "param.sfo"), join(OUT, "sce_sys/param.sfo"));
copyFileSync(join(ROOT, "src/vita/sce_sys/icon0.png"), join(OUT, "sce_sys/icon0.png"));
writeFileSync(join(OUT, "sce_sys/package/head.bin"), headBin(TITLE_ID));
console.log(`Pocket Shelf Updater (${TITLE_ID}) packaged in ${OUT}`);
