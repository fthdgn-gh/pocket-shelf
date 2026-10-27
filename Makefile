# Pocket Shelf's commands. Rust and cargo for all of them; VitaSDK and
# cargo-vita for `build` and `updater`, pspdev for `plugin`.
#
#   make test      core, platform module and tools tests, on the desktop
#   make shots     render every screen to .build/validation/desktop/
#                  (SHOTS="words" renders the shots whose names hold them)
#   make build     the VPK at dist/vita/pocket-shelf.vpk
#   make art       redraw the LiveArea pictures in vita/package/sce_sys/
#   make updater   rebuild the updater app into vita/package/updater/
#   make plugin    rebuild the Adrenaline plugin into vita/package/psp/

VITASDK ?= $(HOME)/vitasdk
PSPDEV ?= $(HOME)/pspdev
TOOLS = cargo run --release --quiet --manifest-path tools/Cargo.toml --

.PHONY: test shots build art updater plugin

test:
	cd core && cargo test
	cd platform-tests && RETROARCH_SAMPLES="$(CURDIR)/.samples/retroarch-playlists" cargo test
	cd tools && cargo test

shots:
	cd desktop && cargo run --release -- $(SHOTS)

build:
	sh vita/build.sh

art:
	$(TOOLS) art

updater:
	PATH="$(VITASDK)/bin:$$PATH" VITASDK="$(VITASDK)" $(MAKE) -C updater
	$(TOOLS) updater

plugin:
	PATH="$(PSPDEV)/bin:$$PATH" PSPDEV="$(PSPDEV)" $(MAKE) -C psp-boot
	cp psp-boot/pocketshelf.prx vita/package/psp/pocketshelf.prx
