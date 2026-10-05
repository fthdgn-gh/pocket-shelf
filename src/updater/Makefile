# Pocket Shelf Updater (GPL-3.0, see LICENSE). Builds eboot.bin and
# param.sfo; `bun run shelf:updater` runs this and packages the result into
# src/vita/updater/, which goes into Pocket Shelf's VPK.

TITLE_ID = POCKTUPDR
TITLE    = Pocket Shelf Updater
TARGET   = updater
OBJS     = main.o sha1.o head.o
BUILD    = ../../.pocket-build/updater

LIBS = -lvita2d -lSceDisplay_stub -lSceGxm_stub -lSceSysmodule_stub -lSceCtrl_stub \
	-lScePgf_stub -lScePvf_stub -lSceCommonDialog_stub -lSceAppMgr_stub -lSceAppUtil_stub \
	-lScePromoterUtil_stub -lfreetype -lpng -ljpeg -lz -lbz2 -lm -lc

PREFIX = arm-vita-eabi
CC     = $(PREFIX)-gcc
CFLAGS = -Wl,-q -Wall -O2 -marm

all: $(BUILD)/eboot.bin $(BUILD)/param.sfo

$(BUILD):
	mkdir -p $@

$(BUILD)/param.sfo: | $(BUILD)
	vita-mksfoex -s TITLE_ID=$(TITLE_ID) -s APP_VER=01.00 "$(TITLE)" $@

# Unsafe: the system's package installer needs the extended permissions.
$(BUILD)/eboot.bin: $(BUILD)/$(TARGET).velf
	vita-make-fself -c $< $@

$(BUILD)/$(TARGET).velf: $(BUILD)/$(TARGET).elf
	vita-elf-create $< $@

$(BUILD)/$(TARGET).elf: $(addprefix $(BUILD)/,$(OBJS))
	$(CC) $(CFLAGS) $^ $(LIBS) -o $@

$(BUILD)/%.o: %.c | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@

# head.bin linked in as _binary_head_bin_start / _binary_head_bin_size.
$(BUILD)/head.o: head.bin | $(BUILD)
	$(PREFIX)-ld -r -b binary -o $@ head.bin

clean:
	rm -rf $(BUILD)

.PHONY: all clean
