# Pocket Shelf boot plugin for Adrenaline (see main.c). Built with the pspdev
# toolchain: `bun run shelf:plugin`, which copies the result to
# src/vita/psp/pocketshelf.prx for the VPK.
TARGET = pocketshelf
OBJS = main.o

BUILD_PRX = 1
PRX_EXPORTS = exports.exp
# A kernel module: it sets the UMD file and boots through SystemCtrlForKernel.
USE_KERNEL_LIBC = 1
USE_KERNEL_LIBS = 1

CFLAGS = -Os -G0 -Wall -Wextra -fno-pic -fno-builtin-printf
LDFLAGS = -nostartfiles
LIBS = -lpspsystemctrl_kernel -lgcc

PSPSDK = $(shell psp-config --pspsdk-path)
include $(PSPSDK)/lib/build.mak
