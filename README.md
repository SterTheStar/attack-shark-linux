<div align="center">
  <img src="ui/src/assets/attack-shark-logo.webp" alt="Attack Shark" width="380" />

  <p>Linux driver and desktop UI for Attack Shark mice.</p>

  <p>
    <img src="https://img.shields.io/badge/Linux-HID-blue?logo=linux&logoColor=white" alt="Linux HID" />
    <img src="https://img.shields.io/badge/Rust-2024-black?logo=rust" alt="Rust" />
    <img src="https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white" alt="Tauri 2" />
    <img src="https://img.shields.io/badge/React-19-149ECA?logo=react&logoColor=white" alt="React 19" />
    <img src="https://img.shields.io/badge/License-GPLv3-blue" alt="GNU GPL v3 License" />
  </p>
</div>

## Supported models

- Attack Shark R1, wired and 2.4 GHz receiver.
- Attack Shark X11, wired and 2.4 GHz receiver.

The X11 and R1 2.4 GHz receivers share USB PID `1d57:fa60`. The driver uses
the documented receiver revisions to identify them: R1 is `11.05`, X11 is
`11.08`. Select the model manually in Settings when a receiver is ambiguous.

## Capabilities

- [X] Passively display wireless battery reports
- [X] Set current polling rate
- [ ] Remap keys
- [X] Set DPI
- [X] Set sleep time
- [X] Set Deepsleep time
- [X] Set key response time
- [X] Ripple control
- [X] Angle Snap
- [ ] Macros

Battery reports are asynchronous. The UI listens for them on the wireless HID
interface and shows the last value received; it does not send a battery query.

## Build requirements

- Rust toolchain (`cargo` and `rustc`)
- `make`
- C compiler, used by the static `hidapi` hidraw backend
- Node.js and npm, for the desktop UI

## Build instructions

```sh
make
```

To run the desktop UI during development:

```sh
npm install --prefix ui
cargo run -- --gui
```

## Release bundles

Build the native Debian and RPM packages plus a portable archive:

```sh
make bundles
```

The artifacts are written to:

```text
src-tauri/target/release/bundle/deb/
src-tauri/target/release/bundle/rpm/
target/release/attack-shark-<version>-linux-x86_64.tar.gz
```

The portable archive works on distributions without a matching native package.
Extract it and run its installer as root:

```sh
tar -xzf attack-shark-<version>-linux-x86_64.tar.gz
cd attack-shark-<version>-linux-x86_64
sudo ./install.sh
```

Use `sudo ./install.sh --prefix /opt/attack-shark` to install the binary under
a different prefix. The installer preserves an existing
`/etc/attack-shark/config.toml` file.

## Configuration

Driver searches for config file by checking following paths:
- $XDG_CONFIG_HOME/attack-shark/config.toml
- $HOME/.config/attack-shark/config.toml
- /etc/attack-shark/config.toml

## HID permissions

The GUI and CLI open `/dev/hidraw*` without root. Install both included udev
rules once, then reconnect the receiver or mouse:

```sh
sudo install -Dm644 99-attack-shark-r1.rules /etc/udev/rules.d/99-attack-shark-r1.rules
sudo install -Dm644 99-attack-shark-x11.rules /etc/udev/rules.d/99-attack-shark-x11.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=usb --attr-match=idVendor=1d57
sudo udevadm trigger --subsystem-match=hidraw
```

The UI displays the same command when the selected model is missing its HID
permission rule.
