#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
version=$(awk -F '"' '/^version = / { print $2; exit }' "$root/src-tauri/Cargo.toml")
package="attack-shark-$version-linux-x86_64"
stage="$root/target/portable/$package"
output="$root/target/release/$package.tar.gz"

rm -rf "$stage"
mkdir -p "$stage/bin" "$stage/icons" "$stage/udev"

cp "$root/src-tauri/target/release/attack-shark" "$stage/bin/attack-shark"
cp "$root/io.attackshark.driver.desktop" "$stage/attack-shark.desktop"
cp "$root/src-tauri/icons/32x32.png" "$stage/icons/32x32.png"
cp "$root/src-tauri/icons/128x128.png" "$stage/icons/128x128.png"
cp "$root/src-tauri/icons/128x128@2x.png" "$stage/icons/256x256.png"
cp "$root/99-attack-shark-r1.rules" "$stage/udev/"
cp "$root/99-attack-shark-x11.rules" "$stage/udev/"
cp "$root/config.toml" "$stage/config.toml"
cp "$root/LICENSE" "$stage/LICENSE"
cp "$root/scripts/install.sh" "$stage/install.sh"
chmod 755 "$stage/bin/attack-shark" "$stage/install.sh"

mkdir -p "$(dirname "$output")"
tar -C "$(dirname "$stage")" -czf "$output" "$package"
printf 'Created %s\n' "$output"
