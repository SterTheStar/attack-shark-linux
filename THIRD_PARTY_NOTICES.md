# Third-Party Notices

## Attack Shark X11 protocol

The Attack Shark X11 USB HID packet formats in `src/x11.rs` are derived from
the documented protocol in
[`HarukaYamamoto0/attack-shark-x11-driver`](https://github.com/HarukaYamamoto0/attack-shark-x11-driver),
licensed under MIT.

The implementation uses native Rust `rusb` transport instead of `node-hid`,
