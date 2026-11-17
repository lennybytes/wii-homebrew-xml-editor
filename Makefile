APP      := wii-meta-editor
PREFIX   ?= /usr/local
DESTDIR  ?=
CARGO    ?= cargo
ARGS     ?=
PORTABLE_TARGET ?= x86_64-unknown-linux-gnu.2.17

BIN      := target/release/$(APP)

.PHONY: all build release portable run test check fmt clippy clean install uninstall help

all: build

build:
	$(CARGO) build

release:
	$(CARGO) build --release

portable:
	$(CARGO) zigbuild --release --target $(PORTABLE_TARGET)

run:
	$(CARGO) run --release -- $(ARGS)

test:
	$(CARGO) test

check:
	$(CARGO) check

fmt:
	$(CARGO) fmt

clippy:
	$(CARGO) clippy -- -D warnings

clean:
	$(CARGO) clean
	rm -rf build dist

install: release
	install -Dm755 $(BIN) $(DESTDIR)$(PREFIX)/bin/$(APP)
	install -Dm644 packaging/$(APP).desktop $(DESTDIR)$(PREFIX)/share/applications/$(APP).desktop
	install -Dm644 packaging/$(APP).svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/$(APP).svg
	install -Dm644 packaging/$(APP).png $(DESTDIR)$(PREFIX)/share/icons/hicolor/256x256/apps/$(APP).png

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/$(APP)
	rm -f $(DESTDIR)$(PREFIX)/share/applications/$(APP).desktop
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/$(APP).svg
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/256x256/apps/$(APP).png

help:
	@echo "Targets:"
	@echo "  build      Build a debug binary"
	@echo "  release    Build an optimized binary"
	@echo "  portable   Build a Linux binary for glibc 2.17+ (needs cargo-zigbuild + zig)"
	@echo "  run        Run the app (ARGS='path/to/meta.xml')"
	@echo "  test       Run unit tests"
	@echo "  check      Type-check the project"
	@echo "  fmt        Format the source code"
	@echo "  clippy     Run clippy with warnings as errors"
	@echo "  install    Install to PREFIX (default: /usr/local)"
	@echo "  uninstall  Remove installed files"
	@echo "  clean      Remove build artifacts"
