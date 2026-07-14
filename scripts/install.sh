#!/bin/sh
set -eu

prefix=/usr/local
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

if [ "${1:-}" = "--prefix" ]; then
    [ $# -eq 2 ] || { printf '%s\n' 'usage: sudo ./install.sh [--prefix PREFIX]' >&2; exit 2; }
    prefix=$2
fi

[ "$(id -u)" -eq 0 ] || { printf '%s\n' 'Run this installer as root, for example: sudo ./install.sh' >&2; exit 1; }

install -Dm755 "$script_dir/bin/attack-shark" "$prefix/bin/attack-shark"
install -Dm644 "$script_dir/attack-shark.desktop" /usr/share/applications/attack-shark.desktop
install -Dm644 "$script_dir/icons/32x32.png" /usr/share/icons/hicolor/32x32/apps/attack-shark.png
install -Dm644 "$script_dir/icons/128x128.png" /usr/share/icons/hicolor/128x128/apps/attack-shark.png
install -Dm644 "$script_dir/icons/256x256.png" /usr/share/icons/hicolor/256x256/apps/attack-shark.png
install -Dm644 "$script_dir/udev/99-attack-shark-r1.rules" /usr/lib/udev/rules.d/99-attack-shark-r1.rules
install -Dm644 "$script_dir/udev/99-attack-shark-x11.rules" /usr/lib/udev/rules.d/99-attack-shark-x11.rules

if [ ! -e /etc/attack-shark/config.toml ]; then
    install -Dm644 "$script_dir/config.toml" /etc/attack-shark/config.toml
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor
fi
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules
    udevadm trigger --subsystem-match=hidraw
fi

printf '%s\n' 'Attack Shark installed. Reconnect the mouse or receiver before opening the application.'
