/*
 * Pocket Shelf console start plugin: a taiHEN user plugin for the home
 * screen (SceShell, the `*main` section of the tai config).
 *
 * It does two things, each switched on in Pocket Shelf (SELECT > Console
 * start), which writes `ux0:data/PocketShelf/autostart.txt`:
 *   boot=1    open Pocket Shelf once at console start;
 *   return=1  open Pocket Shelf again when a game it started closes.
 *
 * Open at console start: SceShell loads this plugin while it starts. The
 * plugin's thread waits until NPXS10079 runs (the system starts it with the
 * home screen), and until BOOT_DELAY_US since the plugin started: VitaXMB
 * found a launch 5 s into the boot crashed the system, while AutoBoot's
 * 10.6 s did not. It then starts POCKTSHLF when it is installed and no other
 * application runs, and starts it a second time when it is not running 3 s
 * later (a first launch can stop at the title's LiveArea page).
 *
 * Return after a game: before Pocket Shelf starts a title it writes
 * `ux0:data/PocketShelf/return.txt` and then ends. The plugin looks for the
 * file every POLL_US. Once another application runs, it waits for every
 * application to close (system ones, NPXS..., do not count), then deletes
 * the file and starts Pocket Shelf. It deletes the file without starting
 * anything when Pocket Shelf runs again first (opened from its bubble), or
 * when no other application started within PENDING_US (the launch failed).
 * A game started from the home screen writes no file, so closing it changes
 * nothing; closing Pocket Shelf itself changes nothing either.
 *
 * Each step goes to `ux0:data/PocketShelf/boot-plugin.log`, started again at
 * each console start and capped at LOG_MAX bytes.
 *
 * Holding L while the console starts makes taiHEN skip every plugin; the
 * plugin's line can then be removed from the tai config.
 *
 * MIT License, as Pocket Shelf.
 */
#include <psp2/appmgr.h>
#include <psp2/io/fcntl.h>
#include <psp2/io/stat.h>
#include <psp2/kernel/clib.h>
#include <psp2/kernel/modulemgr.h>
#include <psp2/kernel/processmgr.h>
#include <psp2/kernel/threadmgr.h>
#include <stdarg.h>

#define SELF "POCKTSHLF"
#define DATA "ux0:data/PocketShelf"
#define SETTINGS DATA "/autostart.txt"
#define MARKER DATA "/return.txt"
#define LOG DATA "/boot-plugin.log"
#define INSTALLED "ux0:app/" SELF "/eboot.bin"
#define URI "psgm:play?titleid=" SELF
/* The flags AutoBoot and VitaOS launch with. */
#define LAUNCH_FLAGS 0xFFFFF

#define BOOT_DELAY_US (12 * 1000 * 1000)
#define HOME_WAIT_US (90 * 1000 * 1000)
#define POLL_US (500 * 1000)
#define PENDING_US (60 * 1000 * 1000)
#define LOG_MAX (32 * 1024)
#define APPS_MAX 16

static SceUInt64 started;
static int log_size;

static void log_line(const char *fmt, ...) {
  char line[192];
  int at = sceClibSnprintf(line, sizeof(line), "%8u ms  ", (unsigned)((sceKernelGetProcessTimeWide() - started) / 1000));
  va_list args;
  va_start(args, fmt);
  at += sceClibVsnprintf(line + at, sizeof(line) - at - 1, fmt, args);
  va_end(args);
  if (at > (int)sizeof(line) - 2)
    at = sizeof(line) - 2;
  line[at++] = '\n';
  if (log_size + at > LOG_MAX)
    return;
  SceUID fd = sceIoOpen(LOG, SCE_O_WRONLY | SCE_O_CREAT | SCE_O_APPEND, 0666);
  if (fd < 0)
    return;
  sceIoWrite(fd, line, at);
  sceIoClose(fd);
  log_size += at;
}

static int exists(const char *path) {
  SceIoStat stat;
  return sceIoGetstat(path, &stat) >= 0;
}

/* `boot=1` and `return=1` in autostart.txt; both off when it is missing. */
static void read_flags(int *boot, int *back) {
  char text[256];
  *boot = 0;
  *back = 0;
  SceUID fd = sceIoOpen(SETTINGS, SCE_O_RDONLY, 0);
  if (fd < 0)
    return;
  int len = sceIoRead(fd, text, sizeof(text) - 1);
  sceIoClose(fd);
  if (len <= 0)
    return;
  text[len] = 0;
  for (int i = 0; i < len; i++) {
    if (i > 0 && text[i - 1] != '\n')
      continue;
    if (sceClibStrncmp(text + i, "boot=1", 6) == 0)
      *boot = 1;
    if (sceClibStrncmp(text + i, "return=1", 8) == 0)
      *back = 1;
  }
}

/* What runs: whether the home screen's NPXS10079 does, Pocket Shelf does,
 * and how many other applications (not NPXS...) do. The names of those go
 * in `others` for the log. */
typedef struct {
  int home;
  int self;
  int apps;
  char others[48];
} Running;

static Running running(void) {
  Running r;
  sceClibMemset(&r, 0, sizeof(r));
  SceInt32 ids[APPS_MAX];
  sceClibMemset(ids, 0, sizeof(ids));
  int count = sceAppMgrGetRunningAppIdListForShell(ids, APPS_MAX);
  if (count < 0)
    count = 0;
  if (count > APPS_MAX)
    count = APPS_MAX;
  int at = 0;
  for (int i = 0; i < count; i++) {
    SceUID pid = sceAppMgrGetProcessIdByAppIdForShell(ids[i]);
    char name[16];
    sceClibMemset(name, 0, sizeof(name));
    if (pid <= 0 || sceAppMgrGetNameById(pid, name) < 0)
      continue;
    if (sceClibStrncmp(name, "NPXS10079", 9) == 0)
      r.home = 1;
    if (sceClibStrncmp(name, SELF, 9) == 0) {
      r.self = 1;
    } else if (sceClibStrncmp(name, "NPXS", 4) != 0) {
      r.apps++;
      if (at + 11 < (int)sizeof(r.others))
        at += sceClibSnprintf(r.others + at, sizeof(r.others) - at, "%s%.9s", at ? " " : "", name);
    }
  }
  return r;
}

static void launch(const char *why) {
  int code = sceAppMgrLaunchAppByUri(LAUNCH_FLAGS, URI);
  log_line("%s: launch %s: 0x%08X", why, SELF, (unsigned)code);
}

static void open_at_start(void) {
  int boot, back;
  read_flags(&boot, &back);
  log_line("start: boot=%d return=%d", boot, back);
  if (!boot)
    return;
  while (!running().home) {
    if (sceKernelGetProcessTimeWide() - started > HOME_WAIT_US) {
      log_line("start: NPXS10079 did not run; not opening");
      return;
    }
    sceKernelDelayThread(100 * 1000);
  }
  log_line("start: home screen running");
  SceUInt64 since = sceKernelGetProcessTimeWide() - started;
  if (since < BOOT_DELAY_US)
    sceKernelDelayThread(BOOT_DELAY_US - since);
  if (!exists(INSTALLED)) {
    log_line("start: " SELF " is not installed");
    return;
  }
  Running r = running();
  if (r.self || r.apps) {
    log_line("start: not opening, running: self=%d others=[%s]", r.self, r.others);
    return;
  }
  launch("start");
  sceKernelDelayThread(3 * 1000 * 1000);
  if (!running().self)
    launch("start, again");
}

/* Return after a game: the marker's states. */
enum { IDLE, PENDING, ARMED };

static void return_after_games(void) {
  int state = IDLE;
  int empty = 0;
  SceUInt64 since = 0;
  for (;;) {
    sceKernelDelayThread(POLL_US);
    if (state == IDLE) {
      if (!exists(MARKER))
        continue;
      int boot, back;
      read_flags(&boot, &back);
      if (!back) {
        sceIoRemove(MARKER);
        log_line("return: marker found, return is off; removed");
        continue;
      }
      state = PENDING;
      since = sceKernelGetProcessTimeWide();
      log_line("return: marker found");
      continue;
    }
    if (!exists(MARKER)) {
      log_line("return: marker gone");
      state = IDLE;
      continue;
    }
    Running r = running();
    if (state == PENDING) {
      if (r.apps && !r.self) {
        state = ARMED;
        empty = 0;
        log_line("return: watching [%s]", r.others);
      } else if (sceKernelGetProcessTimeWide() - since > PENDING_US) {
        sceIoRemove(MARKER);
        state = IDLE;
        log_line("return: nothing started; marker removed");
      }
      continue;
    }
    /* ARMED */
    if (r.self) {
      sceIoRemove(MARKER);
      state = IDLE;
      log_line("return: " SELF " runs again; marker removed");
      continue;
    }
    if (r.apps) {
      empty = 0;
      continue;
    }
    /* Two polls in a row with nothing running: the game has closed. */
    if (++empty < 2)
      continue;
    sceIoRemove(MARKER);
    state = IDLE;
    if (exists(INSTALLED))
      launch("return");
  }
}

static int plugin_thread(SceSize args, void *argp) {
  (void)args;
  (void)argp;
  open_at_start();
  return_after_games();
  return 0;
}

int module_start(SceSize argc, const void *args);
int _start(SceSize argc, const void *args) __attribute__((weak, alias("module_start")));
int module_start(SceSize argc, const void *args) {
  (void)argc;
  (void)args;
  started = sceKernelGetProcessTimeWide();
  /* A new log at each console start. */
  SceUID fd = sceIoOpen(LOG, SCE_O_WRONLY | SCE_O_CREAT | SCE_O_TRUNC, 0666);
  if (fd >= 0)
    sceIoClose(fd);
  log_line("Pocket Shelf console start plugin");
  SceUID thread = sceKernelCreateThread("pocketshelf_boot", plugin_thread, 0xA0, 0x4000, 0, 0, NULL);
  if (thread >= 0)
    sceKernelStartThread(thread, 0, NULL);
  return SCE_KERNEL_START_SUCCESS;
}

int module_stop(SceSize argc, const void *args) {
  (void)argc;
  (void)args;
  return SCE_KERNEL_STOP_SUCCESS;
}
