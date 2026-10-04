/*
 * Pocket Shelf boot plugin: a kernel VSH plugin for Adrenaline.
 *
 * Pocket Shelf starts a PSP or PS1 title that has no bubble by writing a
 * request file and starting Adrenaline. Adrenaline loads this plugin each
 * time its XMB starts, while the XMB's modules are still being started. With
 * no request file, the plugin returns and the XMB runs as usual. With one, it
 * waits until the XMB runs, then deletes the request, checks its age and
 * path, and boots the title the way the XMB does. Booting at once, while the
 * XMB's other modules were still starting, hung the PSP side on the third
 * launch on hardware; booting from the start-module handler, which runs
 * inside the module manager, started nothing at all. So the handler only
 * notes modules, and a thread of the plugin boots. It boots `AFTER_XMB_US`
 * after the first module that starts after `vsh_module` (the XMB's own code
 * is then running and loading its menu, which is not drawn yet), or
 * `XMB_SETTLE_US` after `vsh_module` when no such module comes (the XMB is
 * then up, as when the user picks a game from it; it shows for a moment).
 * The boot itself is done the way the XMB does it:
 *   - a PSP EBOOT: apitype 0x141, key "game";
 *   - a PS1 EBOOT: apitype 0x144, key "pops";
 *   - a disc image (ISO, CSO): the image set as the UMD file, the ISO
 *     driver the user chose in Adrenaline's settings (NP9660 when they cannot
 *     be read), then the image's disc0:/PSP_GAME/SYSDIR/EBOOT.BIN (EBOOT.OLD
 *     for a patched game; BOOT.BIN when the image has one and the user's
 *     "execute BOOT.BIN" setting is on) with key "umdemu", apitype 0x120 in
 *     TheOfficialFloW's Adrenaline and 0x123 with the disc type set to game in
 *     isage's. This is what each Adrenaline's vshctrl does for an ISO picked
 *     in the XMB. isage's is told apart by its sctrlSESetDiscType export.
 *
 * Request file `ms0:/PocketShelf/boot.txt`:
 *   <UTC tick in microseconds when written, as sceRtcGetCurrentTick gives>
 *   <psp, ps1 or iso>
 *   <ms0:/PSP/GAME/<title id>/EBOOT.PBP, or ms0:/ISO/...(.iso|.cso)>
 *   <EBOOT.BIN or EBOOT.OLD, for iso only>
 *   <BOOT.BIN when the image has one, else "-"; for iso only>
 *
 * Every request the plugin finds leaves one line in
 * `ms0:/PocketShelf/boot.log`: its age in seconds, the path and the result.
 *
 * Only two SystemControl functions are imported, both under name-hash NIDs
 * that both Adrenalines export. The rest is looked up with
 * sctrlHENFindFunction, so a function one Adrenaline lacks is a null pointer,
 * not a module that fails to load.
 */

#include <pspkernel.h>
#include <pspiofilemgr.h>
#include <psploadexec_kernel.h>
#include <string.h>

PSP_MODULE_INFO("PocketShelfBoot", 0x1000, 1, 0);

/* SystemCtrlForKernel, in both Adrenalines. */
typedef int (*StartModuleHandler)(void *module);
unsigned int sctrlHENFindFunction(const char *module, const char *library, unsigned int nid);
int sctrlKernelLoadExecVSHWithApitype(int apitype, const char *file, struct SceKernelLoadExecVSHParam *param);
StartModuleHandler sctrlHENSetStartModuleHandler(StartModuleHandler handler);

#define REQUEST_PATH "ms0:/PocketShelf/boot.txt"
#define LOG_PATH "ms0:/PocketShelf/boot.log"
#define GAME_PREFIX "ms0:/PSP/GAME/"
#define EBOOT_SUFFIX "/EBOOT.PBP"
#define ISO_PREFIX "ms0:/ISO/"
#define DISC_SYSDIR "disc0:/PSP_GAME/SYSDIR/"

/* How long after the first module after `vsh_module` the plugin boots; how
 * long after `vsh_module` it boots when no module follows; and how long it
 * waits for `vsh_module` at all. */
#define AFTER_XMB_US (200 * 1000)
#define XMB_SETTLE_US (2 * 1000 * 1000)
#define VSH_WAIT_US (20 * 1000 * 1000)
#define POLL_US (50 * 1000)

/* A request older than this is left over from a launch that never got here;
 * it is dropped. Adrenaline's first start after a reboot takes the longest. */
#define REQUEST_MAX_AGE_US (30ULL * 1000 * 1000)

#define APITYPE_DISC 0x120
#define APITYPE_DISC_EMU_MS1 0x123
#define APITYPE_MS_GAME 0x141
#define APITYPE_MS_PS1 0x144
/* sctrlSESetBootConfFileIndex: NP9660 is 3 in both Adrenalines; the others
 * differ (see iso_driver). */
#define BOOT_NP9660 3
#define UMD_TYPE_GAME 0x10

/* Name-hash NIDs (the first four bytes of the name's SHA-1, little endian). */
#define NID_SET_UMD_FILE 0xB64186D0
#define NID_SET_BOOT_CONF 0x5CB025F0
#define NID_SET_DISC_TYPE 0x31C6160D
#define NID_RTC_GET_CURRENT_TICK 0x3F7AD767
#define NID_SE_GET_CONFIG 0x16C3B7EE

/* The PSP-side settings sctrlSEGetConfig fills, told apart by their magic:
 * TheOfficialFloW's AdrenalineConfig (int fields, umdmode at byte 20,
 * executebootbin at byte 80) and isage's SEConfigEPI (byte fields, umd_mode
 * at byte 11, execute_boot_bin at byte 26). */
#define FLOW_CONFIG_MAGIC_1 0x31483943
#define FLOW_CONFIG_MAGIC_2 0x334F4E33
#define EPI_CONFIG_MAGIC_1 0x192EFC3C
#define EPI_CONFIG_MAGIC_2 0x17BEB6AA

/* The handler that was registered before this one; every module goes on to it. */
static StartModuleHandler previous_handler;
static volatile int vsh_seen;
/* The first module that started after `vsh_module`, once one has. */
static volatile int after_vsh_seen;
static char after_vsh_name[28];

static unsigned char se_config[256];
static char request[768];
static char path[256];
static char boot_path[64];

static void log_line(const char *text) {
	SceUID fd = sceIoOpen(LOG_PATH, PSP_O_WRONLY | PSP_O_CREAT | PSP_O_APPEND, 0777);
	if (fd < 0) {
		return;
	}
	sceIoWrite(fd, text, strlen(text));
	sceIoWrite(fd, "\n", 1);
	sceIoClose(fd);
}

/* Append text to a buffer of `size` bytes, cutting it to fit. */
static void append(char *out, size_t size, const char *text) {
	size_t at = strlen(out);
	while (*text && at + 1 < size) {
		out[at++] = *text++;
	}
	out[at] = 0;
}

/* Append a decimal or hexadecimal number; the plugin has no printf. */
static void append_number(char *out, size_t size, unsigned long long value, int hex) {
	char digits[24];
	char text[28];
	int count = 0;
	int base = hex ? 16 : 10;
	do {
		digits[count++] = "0123456789abcdef"[value % base];
		value /= base;
	} while (value && count < (int)sizeof(digits));
	int at = 0;
	if (hex) {
		text[at++] = '0';
		text[at++] = 'x';
	}
	while (count) {
		text[at++] = digits[--count];
	}
	text[at] = 0;
	append(out, size, text);
}

/* One log line: the request's age (or "?" without a clock), the path, the
 * outcome and a result code when there is one. */
static void log_result(int has_age, unsigned long long age_us, const char *what, int code) {
	char line[400];
	line[0] = 0;
	append(line, sizeof(line), "age=");
	if (has_age) {
		append_number(line, sizeof(line), age_us / 1000000, 0);
		append(line, sizeof(line), "s");
	} else {
		append(line, sizeof(line), "?");
	}
	append(line, sizeof(line), " path=");
	append(line, sizeof(line), path);
	append(line, sizeof(line), " ");
	append(line, sizeof(line), what);
	if (code) {
		append(line, sizeof(line), " code=");
		append_number(line, sizeof(line), (unsigned int)code, 1);
	}
	log_line(line);
}

static int parse_tick(const char *text, unsigned long long *tick) {
	unsigned long long value = 0;
	if (*text < '0' || *text > '9') {
		return 0;
	}
	while (*text >= '0' && *text <= '9') {
		value = value * 10 + (unsigned long long)(*text - '0');
		text++;
	}
	*tick = value;
	return 1;
}

static int starts_with(const char *text, const char *prefix) {
	return strncmp(text, prefix, strlen(prefix)) == 0;
}

static int ends_with_ignoring_case(const char *text, const char *suffix) {
	size_t length = strlen(text), count = strlen(suffix);
	if (length < count) {
		return 0;
	}
	for (size_t at = 0; at < count; at++) {
		char a = text[length - count + at], b = suffix[at];
		if (a >= 'A' && a <= 'Z') {
			a = (char)(a - 'A' + 'a');
		}
		if (a != b) {
			return 0;
		}
	}
	return 1;
}

/* A path under ms0:/PSP/GAME/ that ends in /EBOOT.PBP, one folder deep, with
 * nothing that could leave that folder. */
static int valid_eboot_path(const char *text) {
	size_t length = strlen(text);
	size_t prefix = sizeof(GAME_PREFIX) - 1;
	size_t suffix = sizeof(EBOOT_SUFFIX) - 1;
	if (length <= prefix + suffix || length >= sizeof(path)) {
		return 0;
	}
	if (!starts_with(text, GAME_PREFIX) || strcmp(text + length - suffix, EBOOT_SUFFIX) != 0) {
		return 0;
	}
	for (size_t at = prefix; at < length - suffix; at++) {
		char c = text[at];
		if (c == '/' || c == '\\' || c == ':' || c == '.' || c < ' ') {
			return 0;
		}
	}
	return 1;
}

/* A .iso or .cso file under ms0:/ISO/, with no ".." step, no device and no
 * control characters. */
static int valid_image_path(const char *text) {
	size_t length = strlen(text);
	if (length <= sizeof(ISO_PREFIX) - 1 || length >= sizeof(path) || !starts_with(text, ISO_PREFIX)) {
		return 0;
	}
	if (!ends_with_ignoring_case(text, ".iso") && !ends_with_ignoring_case(text, ".cso")) {
		return 0;
	}
	if (strstr(text, "..") != NULL) {
		return 0;
	}
	for (size_t at = sizeof(ISO_PREFIX) - 1; at < length; at++) {
		char c = text[at];
		if (c == '\\' || c == ':' || (unsigned char)c < ' ') {
			return 0;
		}
	}
	return 1;
}

/* Split the request into up to `count` lines; returns how many it has. */
static int split_lines(char *text, char *lines[], int count) {
	int found = 0;
	while (found < count && *text) {
		lines[found++] = text;
		while (*text && *text != '\n' && *text != '\r') {
			text++;
		}
		while (*text == '\r' || *text == '\n') {
			*text++ = 0;
		}
	}
	return found;
}

static void *system_control(unsigned int nid) {
	return (void *)sctrlHENFindFunction("SystemControl", "SystemCtrlForKernel", nid);
}

static unsigned int config_word(int at) {
	return se_config[at] | se_config[at + 1] << 8 | se_config[at + 2] << 16 | (unsigned int)se_config[at + 3] << 24;
}

/* The boot config index of the user's ISO driver and whether they asked for
 * BOOT.BIN, from the PSP-side settings. Each Adrenaline numbers its drivers
 * its own way: TheOfficialFloW's settings say Inferno 0, March33 1, NP9660 2,
 * booted as 1, 2, 3; isage's say Inferno 0, March33 1, ME 2, NP9660 3,
 * booted as 4, 2, 5, 3. Returns 0 when the settings cannot be read. */
static int iso_driver(int *boot_bin) {
	int (*get_config)(void *) = system_control(NID_SE_GET_CONFIG);
	*boot_bin = 0;
	if (!get_config) {
		return 0;
	}
	memset(se_config, 0, sizeof(se_config));
	if (get_config(se_config) < 0) {
		return 0;
	}
	if (config_word(0) == FLOW_CONFIG_MAGIC_1 && config_word(4) == FLOW_CONFIG_MAGIC_2) {
		static const int boot_index[] = {1, 2, 3};
		unsigned int mode = config_word(20);
		*boot_bin = config_word(80) != 0;
		return mode < 3 ? boot_index[mode] : 0;
	}
	if (config_word(0) == EPI_CONFIG_MAGIC_1 && config_word(4) == EPI_CONFIG_MAGIC_2) {
		static const int boot_index[] = {4, 2, 5, 3};
		unsigned int mode = se_config[11];
		*boot_bin = se_config[26] != 0;
		return mode < 4 ? boot_index[mode] : 0;
	}
	return 0;
}

/* Boot a disc image as the XMB boots one from its game list. `boot_file` is
 * EBOOT.BIN or EBOOT.OLD; `has_boot_bin` says the image has a BOOT.BIN. */
static int boot_image(const char *boot_file, int has_boot_bin) {
	void (*set_umd_file)(const char *) = system_control(NID_SET_UMD_FILE);
	int (*set_boot_conf)(int) = system_control(NID_SET_BOOT_CONF);
	/* Present in isage's Adrenaline only. */
	int (*set_disc_type)(int) = system_control(NID_SET_DISC_TYPE);
	if (!set_umd_file || !set_boot_conf) {
		return -1;
	}
	int wants_boot_bin = 0;
	int driver = iso_driver(&wants_boot_bin);
	if (strcmp(boot_file, "EBOOT.OLD") != 0 && wants_boot_bin && has_boot_bin) {
		boot_file = "BOOT.BIN";
	}
	char note[48];
	note[0] = 0;
	append(note, sizeof(note), "driver=");
	append_number(note, sizeof(note), driver ? (unsigned int)driver : BOOT_NP9660, 0);
	append(note, sizeof(note), driver ? " " : " (default) ");
	append(note, sizeof(note), boot_file);
	log_line(note);

	set_umd_file(path);
	if (set_disc_type) {
		set_disc_type(UMD_TYPE_GAME);
	}
	set_boot_conf(driver ? driver : BOOT_NP9660);

	boot_path[0] = 0;
	append(boot_path, sizeof(boot_path), DISC_SYSDIR);
	append(boot_path, sizeof(boot_path), boot_file);
	struct SceKernelLoadExecVSHParam param;
	memset(&param, 0, sizeof(param));
	param.size = sizeof(param);
	param.args = strlen(boot_path) + 1;
	param.argp = boot_path;
	param.key = "umdemu";
	int apitype = set_disc_type ? APITYPE_DISC_EMU_MS1 : APITYPE_DISC;
	return sctrlKernelLoadExecVSHWithApitype(apitype, boot_path, &param);
}

static int boot_eboot(int ps1) {
	struct SceKernelLoadExecVSHParam param;
	memset(&param, 0, sizeof(param));
	param.size = sizeof(param);
	param.args = strlen(path) + 1;
	param.argp = path;
	param.key = ps1 ? "pops" : "game";
	return sctrlKernelLoadExecVSHWithApitype(ps1 ? APITYPE_MS_PS1 : APITYPE_MS_GAME, path, &param);
}

/* Read, delete and act on the request. Returns only when nothing was booted. */
static void handle_request(void) {
	SceUID fd = sceIoOpen(REQUEST_PATH, PSP_O_RDONLY, 0);
	if (fd < 0) {
		return;
	}
	int size = sceIoRead(fd, request, sizeof(request) - 1);
	sceIoClose(fd);
	/* Deleted before anything else, so a request is acted on once at most. */
	sceIoRemove(REQUEST_PATH);
	path[0] = 0;
	if (size <= 0) {
		log_line("empty request");
		return;
	}
	request[size] = 0;

	char *lines[5];
	int count = split_lines(request, lines, 5);
	unsigned long long written = 0;
	if (count < 3 || !parse_tick(lines[0], &written)) {
		log_line("malformed request");
		return;
	}
	append(path, sizeof(path), lines[2]);

	/* The clock is looked up too; without it the age is not checked. */
	int (*get_tick)(u64 *) = (void *)sctrlHENFindFunction("sceRTC_Service", "sceRtc", NID_RTC_GET_CURRENT_TICK);
	u64 now = 0;
	int has_age = get_tick && get_tick(&now) >= 0;
	unsigned long long age = has_age && now > written ? now - written : 0;
	if (has_age && age > REQUEST_MAX_AGE_US) {
		log_result(has_age, age, "stale", 0);
		return;
	}

	int image = strcmp(lines[1], "iso") == 0;
	int ps1 = strcmp(lines[1], "ps1") == 0;
	if (!image && !ps1 && strcmp(lines[1], "psp") != 0) {
		log_result(has_age, age, "bad kind", 0);
		return;
	}
	if (image ? !valid_image_path(path) : !valid_eboot_path(path)) {
		log_result(has_age, age, "bad path", 0);
		return;
	}
	const char *boot_file = "EBOOT.BIN";
	if (image && count >= 4) {
		if (strcmp(lines[3], "EBOOT.OLD") == 0) {
			boot_file = "EBOOT.OLD";
		} else if (strcmp(lines[3], "EBOOT.BIN") != 0) {
			log_result(has_age, age, "bad boot file", 0);
			return;
		}
	}
	SceIoStat stat;
	if (sceIoGetstat(path, &stat) < 0) {
		log_result(has_age, age, "missing", 0);
		return;
	}

	log_result(has_age, age, image ? "boot iso" : ps1 ? "boot ps1" : "boot psp", 0);
	int has_boot_bin = image && count >= 5 && strcmp(lines[4], "BOOT.BIN") == 0;
	int code = image ? boot_image(boot_file, has_boot_bin) : boot_eboot(ps1);
	/* Only reached when the boot was refused. */
	log_result(has_age, age, "refused", code);
}

/* Called by SystemControl, inside the module manager, before each module
 * starts; nothing may be booted from here. The module's name is at offset 8
 * of its SceModule2. The handler stays registered and passes every module
 * on: putting the previous one back could drop a handler another plugin
 * registered after this one. */
static int on_module_start(void *module) {
	const char *name = (const char *)module + 8;
	if (!vsh_seen) {
		if (strcmp(name, "vsh_module") == 0) {
			vsh_seen = 1;
		}
	} else if (!after_vsh_seen) {
		after_vsh_name[0] = 0;
		append(after_vsh_name, sizeof(after_vsh_name), name);
		after_vsh_seen = 1;
	}
	return previous_handler ? previous_handler(module) : 0;
}

/* Wait for the XMB, then act on the request. When the XMB never comes, the
 * request stays, and Pocket Shelf deletes it at its next start. */
static int boot_thread(SceSize args, void *argp) {
	(void)args;
	(void)argp;
	for (unsigned int waited = 0; !vsh_seen && waited < VSH_WAIT_US; waited += POLL_US) {
		sceKernelDelayThread(POLL_US);
	}
	if (!vsh_seen) {
		log_line("vsh_module not seen; request left");
		return sceKernelExitDeleteThread(0);
	}
	unsigned int waited = 0;
	for (; !after_vsh_seen && waited < XMB_SETTLE_US; waited += POLL_US) {
		sceKernelDelayThread(POLL_US);
	}
	char line[64];
	line[0] = 0;
	if (after_vsh_seen) {
		sceKernelDelayThread(AFTER_XMB_US);
		append(line, sizeof(line), "trigger=after ");
		append(line, sizeof(line), after_vsh_name);
	} else {
		append(line, sizeof(line), "trigger=fallback");
	}
	log_line(line);
	handle_request();
	return sceKernelExitDeleteThread(0);
}

int module_start(SceSize args, void *argp) {
	(void)args;
	(void)argp;
	SceIoStat stat;
	if (sceIoGetstat(REQUEST_PATH, &stat) < 0) {
		return 0;
	}
	previous_handler = sctrlHENSetStartModuleHandler(on_module_start);
	SceUID thread = sceKernelCreateThread("PocketShelfBoot", boot_thread, 0x18, 0x1000, 0, NULL);
	if (thread >= 0) {
		sceKernelStartThread(thread, 0, NULL);
	}
	return 0;
}

int module_stop(SceSize args, void *argp) {
	(void)args;
	(void)argp;
	return 0;
}
