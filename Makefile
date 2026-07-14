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
	mkdir -p "${DESTDIR}etc"
	install -Dm755  driver "${DESTDIR}usr/bin/attack-shark"
	install -Dm644 99-attack-shark-r1.rules "${DESTDIR}usr/lib/udev/rules.d"
	install -Dm644 99-attack-shark-x11.rules "${DESTDIR}usr/lib/udev/rules.d"
	install -Dm644 --target-directory="${DESTDIR}etc" attack-shark.ini
.PHONY: uninstall
uninstall:
	rm "${DESTDIR}usr/bin/attack-shark" "${DESTDIR}etc/attack-shark.ini" "${DESTDIR}usr/lib/udev/rules.d/99-attack-shark-r1.rules" "${DESTDIR}usr/lib/udev/rules.d/99-attack-shark-x11.rules"

.PHONY: clean
clean:
	rm -f driver
	cargo clean
