driver: Cargo.toml Cargo.lock src/*.rs
	cargo build
	cp target/debug/attack-shark $@
.PHONY: release 
release: Cargo.toml Cargo.lock src/*.rs
	cargo build --release
	cp target/release/attack-shark driver
DESTDIR ?= /
.PHONY: install
install: ./driver 
	mkdir -p "$(DESTDIR)usr/bin"
	mkdir -p "$(DESTDIR)usr/lib/udev/rules.d"
	mkdir -p "${DESTDIR}etc/attack-shark"
	mkdir -p "${DESTDIR}usr/share/applications"
	mkdir -p "${DESTDIR}usr/share/icons/hicolor/256x256/apps"
	install -Dm755  driver "${DESTDIR}usr/bin/attack-shark"
	install -Dm644 99-attack-shark-r1.rules "${DESTDIR}usr/lib/udev/rules.d"
	install -Dm644 99-attack-shark-x11.rules "${DESTDIR}usr/lib/udev/rules.d"
	install -Dm644 --target-directory="${DESTDIR}etc/attack-shark" config.toml
	install -Dm644 io.attackshark.driver.desktop "${DESTDIR}usr/share/applications/io.attackshark.driver.desktop"
	install -Dm644 src-tauri/icons/128x128@2x.png "${DESTDIR}usr/share/icons/hicolor/256x256/apps/io.attackshark.driver.png"
.PHONY: uninstall
uninstall:
	rm "${DESTDIR}usr/bin/attack-shark" "${DESTDIR}etc/attack-shark/config.toml" "${DESTDIR}usr/lib/udev/rules.d/99-attack-shark-r1.rules" "${DESTDIR}usr/lib/udev/rules.d/99-attack-shark-x11.rules" "${DESTDIR}usr/share/applications/io.attackshark.driver.desktop" "${DESTDIR}usr/share/icons/hicolor/256x256/apps/io.attackshark.driver.png"

.PHONY: clean
clean:
	rm -f driver
	cargo clean
