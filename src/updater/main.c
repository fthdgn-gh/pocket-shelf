/*
  Pocket Shelf Updater
  Copyright (C) 2026 fthdgn

  Installs a Pocket Shelf release that Pocket Shelf downloaded and unpacked,
  then starts Pocket Shelf again. Pocket Shelf cannot install itself: the
  system's package installer replaces ux0:app/POCKTSHLF, which the running app
  is mounted from.

  makeHeadBin, fpkg_hmac, promoteApp, the PAF loader and head.bin come from
  VitaShell (Copyright (C) 2015-2018, TheFloW), sha1.c from Brad Conte.

  This program is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  This program is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

#include <psp2/appmgr.h>
#include <psp2/io/fcntl.h>
#include <psp2/io/stat.h>
#include <psp2/kernel/processmgr.h>
#include <psp2/kernel/threadmgr.h>
#include <psp2/promoterutil.h>
#include <psp2/sysmodule.h>
#include <vita2d.h>

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sha1.h"

#define APP_ID "POCKTSHLF"
#define UPDATE_DIR "ux0:data/PocketShelf/update"
#define PACKAGE_DIR UPDATE_DIR "/pkg"
#define HEAD_BIN PACKAGE_DIR "/sce_sys/package/head.bin"
#define RESULT_FILE UPDATE_DIR "/result.txt"

extern unsigned char _binary_head_bin_start;
extern unsigned char _binary_head_bin_size;

// What the screen says; the worker thread changes it.
static char status[128] = "Checking the update...";
static volatile int finished = 0;

static uint32_t be32(const uint8_t *p) {
  return ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) | ((uint32_t)p[2] << 8) | p[3];
}

static int read_file(const char *path, uint8_t **out, int *size) {
  SceUID fd = sceIoOpen(path, SCE_O_RDONLY, 0);
  if (fd < 0)
    return fd;
  int length = sceIoLseek(fd, 0, SCE_SEEK_END);
  sceIoLseek(fd, 0, SCE_SEEK_SET);
  if (length <= 0 || length > 64 * 1024) {
    sceIoClose(fd);
    return -1;
  }
  uint8_t *buffer = calloc(1, length + 1);
  int read = sceIoRead(fd, buffer, length);
  sceIoClose(fd);
  if (read != length) {
    free(buffer);
    return -1;
  }
  *out = buffer;
  *size = length;
  return 0;
}

static int write_file(const char *path, const void *data, int size) {
  SceUID fd = sceIoOpen(path, SCE_O_WRONLY | SCE_O_CREAT | SCE_O_TRUNC, 0777);
  if (fd < 0)
    return fd;
  int written = sceIoWrite(fd, data, size);
  sceIoClose(fd);
  return written == size ? 0 : -1;
}

// The UTF-8 string `key` of a param.sfo, copied into `out`.
static int sfo_string(const uint8_t *sfo, int size, const char *key, char *out, int out_size) {
  if (size < 20 || memcmp(sfo, "\0PSF", 4) != 0)
    return -1;
  uint32_t keys = *(const uint32_t *)&sfo[8];
  uint32_t values = *(const uint32_t *)&sfo[12];
  uint32_t count = *(const uint32_t *)&sfo[16];
  for (uint32_t i = 0; i < count && 20 + i * 16 + 16 <= (uint32_t)size; i++) {
    const uint8_t *entry = &sfo[20 + i * 16];
    uint32_t name = keys + *(const uint16_t *)&entry[0];
    uint32_t value = values + *(const uint32_t *)&entry[12];
    if (name >= (uint32_t)size || value >= (uint32_t)size)
      return -1;
    if (strcmp((const char *)&sfo[name], key) == 0) {
      snprintf(out, out_size, "%s", (const char *)&sfo[value]);
      return 0;
    }
  }
  return -1;
}

static void fpkg_hmac(const uint8_t *data, unsigned int len, uint8_t hmac[16]) {
  SHA1_CTX ctx;
  uint8_t sha1[20];
  uint8_t buf[64];

  sha1_init(&ctx);
  sha1_update(&ctx, data, len);
  sha1_final(&ctx, sha1);

  memset(buf, 0, 64);
  memcpy(&buf[0], &sha1[4], 8);
  memcpy(&buf[8], &sha1[4], 8);
  memcpy(&buf[16], &sha1[12], 4);
  buf[20] = sha1[16];
  buf[21] = sha1[1];
  buf[22] = sha1[2];
  buf[23] = sha1[3];
  memcpy(&buf[24], &buf[16], 8);

  sha1_init(&ctx);
  sha1_update(&ctx, buf, 64);
  sha1_final(&ctx, sha1);
  memcpy(hmac, sha1, 16);
}

// The package header the installer expects in sce_sys/package/head.bin.
static int make_head_bin(const char *title_id, const char *content_id) {
  int size = (int)&_binary_head_bin_size;
  uint8_t *head_bin = malloc(size);
  memcpy(head_bin, &_binary_head_bin_start, size);

  char full_title_id[48];
  snprintf(full_title_id, sizeof(full_title_id), "EP9000-%s_00-0000000000000000", title_id);
  strncpy((char *)&head_bin[0x30], strlen(content_id) > 0 ? content_id : full_title_id, 48);

  uint8_t hmac[16];
  uint32_t len = be32(&head_bin[0xD0]);
  fpkg_hmac(&head_bin[0], len, hmac);
  memcpy(&head_bin[len], hmac, 16);

  uint32_t off = be32(&head_bin[0x8]);
  len = be32(&head_bin[0x10]);
  uint32_t out = be32(&head_bin[0xD4]);
  fpkg_hmac(&head_bin[off], len - 64, hmac);
  memcpy(&head_bin[out], hmac, 16);

  len = be32(&head_bin[0xE8]);
  fpkg_hmac(&head_bin[0], len, hmac);
  memcpy(&head_bin[len], hmac, 16);

  sceIoMkdir(PACKAGE_DIR "/sce_sys/package", 0777);
  sceIoRemove(HEAD_BIN);
  int res = write_file(HEAD_BIN, head_bin, size);
  free(head_bin);
  return res;
}

static int load_paf(void) {
  static uint32_t argp[] = { 0x180000, -1, -1, 1, -1, -1 };
  int result = -1;
  // VitaShell passes { sizeof, &result, -1, -1 } as four words.
  SceSysmoduleOpt opt = { .flags = sizeof(opt), .result = &result, .unused = { -1, -1 } };
  return sceSysmoduleLoadModuleInternalWithArg(SCE_SYSMODULE_INTERNAL_PAF, sizeof(argp), argp, &opt);
}

static int unload_paf(void) {
  SceSysmoduleOpt opt = { 0 };
  return sceSysmoduleUnloadModuleInternalWithArg(SCE_SYSMODULE_INTERNAL_PAF, 0, NULL, &opt);
}

static int promote(const char *path) {
  int res = load_paf();
  if (res < 0)
    return res;
  res = sceSysmoduleLoadModuleInternal(SCE_SYSMODULE_INTERNAL_PROMOTER_UTIL);
  if (res < 0)
    return res;
  res = scePromoterUtilityInit();
  if (res < 0)
    return res;
  res = scePromoterUtilityPromotePkgWithRif(path, 1);
  scePromoterUtilityExit();
  sceSysmoduleUnloadModuleInternal(SCE_SYSMODULE_INTERNAL_PROMOTER_UTIL);
  unload_paf();
  return res;
}

// "ok <APP_VER>" or "error <step> 0x<code>", read by Pocket Shelf at its next start.
static void report(const char *line) {
  write_file(RESULT_FILE, line, strlen(line));
}

static int work(SceSize args, void *argp) {
  (void)args;
  (void)argp;
  char line[96];
  uint8_t *sfo = NULL;
  int size = 0;
  char title_id[16] = "", content_id[48] = "", version[16] = "";

  if (read_file(PACKAGE_DIR "/sce_sys/param.sfo", &sfo, &size) < 0) {
    report("error package missing");
    snprintf(status, sizeof(status), "No update to install.");
    goto done;
  }
  sfo_string(sfo, size, "TITLE_ID", title_id, sizeof(title_id));
  sfo_string(sfo, size, "CONTENT_ID", content_id, sizeof(content_id));
  sfo_string(sfo, size, "APP_VER", version, sizeof(version));
  free(sfo);
  if (strcmp(title_id, APP_ID) != 0) {
    report("error title id");
    snprintf(status, sizeof(status), "The update is not Pocket Shelf (%s).", title_id);
    goto done;
  }

  if (make_head_bin(title_id, content_id) < 0) {
    report("error head.bin");
    snprintf(status, sizeof(status), "The update could not be prepared.");
    goto done;
  }

  snprintf(status, sizeof(status), "Installing Pocket Shelf %s...", version);
  int res = promote(PACKAGE_DIR);
  if (res < 0) {
    snprintf(line, sizeof(line), "error install 0x%08X", (unsigned)res);
    report(line);
    snprintf(status, sizeof(status), "Installing failed (0x%08X).", (unsigned)res);
    goto done;
  }
  snprintf(line, sizeof(line), "ok %s", version);
  report(line);
  snprintf(status, sizeof(status), "Pocket Shelf %s is installed. Starting it...", version);

done:
  finished = 1;
  return sceKernelExitDeleteThread(0);
}

int main(void) {
  sceAppMgrDestroyOtherApp();

  vita2d_init();
  vita2d_set_clear_color(RGBA8(0x14, 0x16, 0x1c, 0xff));
  vita2d_pgf *font = vita2d_load_default_pgf();

  SceUID thread = sceKernelCreateThread("updater", work, 0x10000100, 0x10000, 0, 0, NULL);
  sceKernelStartThread(thread, 0, NULL);

  // Keep drawing while the install runs, then show the outcome for two seconds.
  int frames_after = 0;
  while (frames_after < 120) {
    if (finished)
      frames_after++;
    vita2d_start_drawing();
    vita2d_clear_screen();
    vita2d_pgf_draw_text(font, 40, 60, RGBA8(0xff, 0xff, 0xff, 0xff), 1.2f, "Pocket Shelf Updater");
    vita2d_pgf_draw_text(font, 40, 110, RGBA8(0xc8, 0xcc, 0xd4, 0xff), 1.0f, status);
    vita2d_end_drawing();
    vita2d_swap_buffers();
  }

  vita2d_free_pgf(font);
  vita2d_fini();

  // Sent twice, 10 ms apart, as VitaShell's updater does.
  sceAppMgrLaunchAppByUri(0xFFFFF, "psgm:play?titleid=" APP_ID);
  sceKernelDelayThread(10000);
  sceAppMgrLaunchAppByUri(0xFFFFF, "psgm:play?titleid=" APP_ID);
  sceKernelExitProcess(0);
  return 0;
}
