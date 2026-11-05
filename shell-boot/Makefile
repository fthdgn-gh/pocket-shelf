# Pocket Shelf console start plugin (see main.c), a taiHEN user plugin for
# the home screen. Built with VitaSDK: `make shell-plugin`, which copies the
# result to vita/package/tai/pocketshelf_boot.suprx for the VPK.
TARGET = pocketshelf_boot
BUILD  = ../.build/shell-boot

PREFIX = arm-vita-eabi
CC     = $(PREFIX)-gcc
CFLAGS = -Wl,-q -Wall -Wextra -O2 -nostartfiles
LIBS   = -lSceAppMgr_stub -lSceIofilemgr_stub -lSceLibKernel_stub -lSceKernelThreadMgr_stub -lSceProcessmgr_stub

all: $(BUILD)/$(TARGET).suprx

$(BUILD):
	mkdir -p $@

$(BUILD)/$(TARGET).suprx: $(BUILD)/$(TARGET).velf
	vita-make-fself -c $< $@

$(BUILD)/$(TARGET).velf: $(BUILD)/$(TARGET).elf exports.yml
	vita-elf-create -e exports.yml $< $@

$(BUILD)/$(TARGET).elf: main.c | $(BUILD)
	$(CC) $(CFLAGS) $< $(LIBS) -o $@

clean:
	rm -rf $(BUILD)

.PHONY: all clean
