# Attack Shark driver
Capabilities:
- [X] Get battery charge
- [X] Set current polling rate
- [ ] Remap keys
- [X] Set DPI
- [X] Set sleep time
- [X] Set Deepsleep time
- [X] Set key response time
- [X] Ripple control
- [X] Angle Snap
- [ ] Macros
# Build requirements
    - Rust toolchain (cargo and rustc)
    - make
    - C compiler, used to build the vendored libusb dependency
# Build instructions
```sh
make
```

# Installation
## Arch-based distros
```sh
git clone https://github.com/xb-bx/attack-shark-r1-driver --recursive
cd attack-shark-r1-driver
makepkg -si
```
## Other
```sh
git clone https://github.com/xb-bx/attack-shark-r1-driver --recursive
sudo make install
```

# Configuration

Driver searches for config file by checking following paths:
- $XDG_CONFIG_HOME/attack-shark.ini
- $HOME/.config/attack-shark.ini
- /etc/attack-shark.ini

## Default configuration [attack-shark.ini](attack-shark.ini)

# USB permissions

The GUI and CLI need the included udev rule to open the mouse without root.
Install it once, then reconnect the receiver or mouse:

```sh
sudo install -Dm644 99-attack-shark-r1.rules /etc/udev/rules.d/99-attack-shark-r1.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=usb --attr-match=idVendor=1d57
```
