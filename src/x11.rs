//! Native USB HID implementation of the documented Attack Shark X11 protocol.

use std::time::Duration;

use hidapi::{HidApi, HidDevice};

use crate::{
    config::{Config, LedMode, PollingRate},
    device::configuration_devices,
    error::{DriverError, Result},
};

const WIRED_PRODUCT_ID: u16 = 0xfa55;
const WIRELESS_PRODUCT_ID: u16 = 0xfa60;
const X11_WIRELESS_VERSION: u16 = 0x1108;
const PACKET_DELAY: Duration = Duration::from_millis(300);

pub struct X11Device {
    device: HidDevice,
    wired: bool,
}

impl X11Device {
    pub fn open(allow_unknown_adapter: bool) -> Result<Self> {
        let api = HidApi::new()?;

        for product_id in [WIRED_PRODUCT_ID, WIRELESS_PRODUCT_ID] {
            for device in configuration_devices(&api) {
                if device.product_id() != product_id {
                    continue;
                }
                if product_id == WIRELESS_PRODUCT_ID
                    && !supports_x11_adapter(device.release_number(), allow_unknown_adapter)
                {
                    continue;
                }
                return Ok(Self {
                    device: device.open_device(&api)?,
                    wired: product_id == WIRED_PRODUCT_ID,
                });
            }
        }
        Err(DriverError::DeviceNotFound)
    }

    pub const fn is_wired(&self) -> bool {
        self.wired
    }

    pub fn set_polling_rate(&mut self, rate: PollingRate) -> Result<()> {
        self.send(0x0306, &polling_packet(rate))
    }

    pub fn set_preferences(&mut self, config: &Config) -> Result<()> {
        let packet = preferences_packet(config);
        let length = if self.wired { 13 } else { 15 };
        self.send(0x0305, &packet[..length])
    }

    pub fn set_dpis(&mut self, config: &Config) -> Result<()> {
        let packet = dpi_packet(config)?;
        let length = if self.wired { 52 } else { 56 };
        self.send(0x0304, &packet[..length])
    }

    fn send(&mut self, value: u16, packet: &[u8]) -> Result<()> {
        debug_assert_eq!(packet[0], (value & 0xff) as u8);
        self.device.send_feature_report(packet)?;
        // The documented X11 protocol has no command acknowledgement.
        std::thread::sleep(PACKET_DELAY);
        Ok(())
    }
}

fn supports_x11_adapter(release_number: u16, allow_unknown_adapter: bool) -> bool {
    allow_unknown_adapter || release_number == X11_WIRELESS_VERSION
}

fn polling_packet(rate: PollingRate) -> [u8; 9] {
    let value = match rate {
        PollingRate::Hz125 => 0x08,
        PollingRate::Hz250 => 0x04,
        PollingRate::Hz500 => 0x02,
        PollingRate::Hz1000 => 0x01,
    };
    [0x06, 0x09, 0x01, value, 0xff - value, 0, 0, 0, 0]
}

fn preferences_packet(config: &Config) -> [u8; 15] {
    let mut packet = [0_u8; 15];
    packet[0..3].copy_from_slice(&[0x05, 0x0f, 0x01]);
    packet[3] = match config.led_mode {
        LedMode::Disabled => 0,
        LedMode::Static => 0x10,
        LedMode::Breathing => 0x20,
        LedMode::Neon => 0x30,
        LedMode::ColorBreathing => 0x40,
        LedMode::StaticDpi => 0x50,
        LedMode::BreathingDpi => 0x60,
    };
    let deep_sleep_bucket = (config.deep_sleep_time - 1) / 16;
    packet[4] = deep_sleep_bucket << 4 | (6 - config.led_speed);
    packet[5] = 0x08_u8.wrapping_add(config.deep_sleep_time.wrapping_mul(16));
    packet[6..9].copy_from_slice(&config.led_color);
    packet[9] = (config.sleep_time * 2.0).round() as u8;
    packet[10] = (config.key_response_time - 4) / 2 + 2;
    packet[11] = config
        .led_color
        .iter()
        .filter(|channel| **channel >= 0x64)
        .count() as u8
        + u8::from(config.led_mode == LedMode::BreathingDpi);
    packet[12] = packet[3..=10]
        .iter()
        .fold(0_u8, |sum, byte| sum.wrapping_add(*byte));
    packet
}

fn dpi_packet(config: &Config) -> Result<[u8; 56]> {
    let mut packet = [
        0x04, 0x38, 0x01, 0, 0, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0xff, 0, 0, 0, 0xff, 0, 0, 0, 0xff, 0xff, 0xff, 0, 0, 0xff, 0xff, 0xff, 0, 0xff, 0xff,
        0x40, 0, 0xff, 0xff, 0xff, 2, 0, 0, 0, 0, 0, 0,
    ];
    let mut stage_mask = 0_u8;
    for (index, dpi) in config.dpis.iter().enumerate() {
        packet[8 + index] = x11_dpi_value(*dpi)?;
        if *dpi > 12_000 {
            stage_mask |= 1 << index;
        }
        if (10_100..=12_000).contains(dpi) || (20_100..=22_000).contains(dpi) {
            packet[16 + index] = 1;
        }
    }
    packet[3] = u8::from(config.angle_snap);
    packet[4] = u8::from(config.ripple_control);
    packet[6] = stage_mask;
    packet[7] = stage_mask;
    packet[24] = config.active_dpi;
    for (index, color) in config.dpi_colors.iter().enumerate() {
        packet[25 + index * 3..28 + index * 3].copy_from_slice(color);
    }
    // Continuous lighting follows in report 0x05; avoid the one-shot flash.
    packet[49] = 0;
    let checksum = packet[3..=49]
        .iter()
        .fold(0_u16, |sum, byte| sum.wrapping_add(*byte as u16));
    packet[50..52].copy_from_slice(&checksum.to_be_bytes());
    Ok(packet)
}

fn x11_dpi_value(dpi: u16) -> Result<u8> {
    if dpi == 50 {
        return Ok(0x01);
    }
    if (100..=10_000).contains(&dpi) && dpi.is_multiple_of(100) {
        return Ok(crate::protocol::dpi_value(dpi));
    }
    if (150..10_000).contains(&dpi) && !dpi.is_multiple_of(100) && dpi.is_multiple_of(50) {
        return Ok(crate::protocol::dpi_value(dpi + 50).saturating_sub(1));
    }
    if (10_100..=18_000).contains(&dpi) && dpi.is_multiple_of(100) {
        return Ok(crate::protocol::dpi_value(dpi));
    }
    const HIGH_RANGE: [u8; 20] = [
        0xd4, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xdb, 0xdd, 0xde, 0xdf, 0xe0, 0xe1, 0xe3, 0xe4, 0xe5,
        0xe6, 0xe7, 0xe8, 0xea, 0xeb,
    ];
    if (18_100..=20_000).contains(&dpi) && dpi.is_multiple_of(100) {
        return Ok(HIGH_RANGE[((dpi - 18_100) / 100) as usize]);
    }
    const EXTENDED_RANGE: [u8; 20] = [
        0xeb, 0x76, 0x76, 0x77, 0x77, 0x79, 0x79, 0x7a, 0x7a, 0x7b, 0x7b, 0x7c, 0x7c, 0x7d, 0x7d,
        0x7f, 0x7f, 0x80, 0x80, 0x81,
    ];
    if (20_100..=22_000).contains(&dpi) && dpi.is_multiple_of(100) {
        return Ok(EXTENDED_RANGE[((dpi - 20_100) / 100) as usize]);
    }
    Err(DriverError::InvalidConfig(format!(
        "X11 DPI must be 50-step up to 10000 or 100-step up to 22000; got {dpi}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            polling_rate: PollingRate::Hz500,
            dpis: [800, 1600, 3200, 4000, 5000, 12_000],
            active_dpi: 3,
            sleep_time: 6.0,
            deep_sleep_time: 12,
            key_response_time: 4,
            angle_snap: false,
            ripple_control: false,
            led_mode: LedMode::Disabled,
            led_color: [0, 255, 0],
            dpi_colors: [
                [255, 0, 0],
                [0, 255, 0],
                [0, 255, 255],
                [255, 0, 0],
                [0, 255, 255],
                [64, 0, 255],
            ],
            led_speed: 3,
        }
    }

    #[test]
    fn encodes_documented_x11_dpi_values() {
        assert_eq!(x11_dpi_value(50).unwrap(), 0x01);
        assert_eq!(x11_dpi_value(3_150).unwrap(), 0x4a);
        assert_eq!(x11_dpi_value(18_100).unwrap(), 0xd4);
        assert_eq!(x11_dpi_value(22_000).unwrap(), 0x81);
    }

    #[test]
    fn builds_documented_polling_packets() {
        assert_eq!(
            polling_packet(PollingRate::Hz125),
            [0x06, 0x09, 0x01, 0x08, 0xf7, 0, 0, 0, 0]
        );
        assert_eq!(
            polling_packet(PollingRate::Hz1000),
            [0x06, 0x09, 0x01, 0x01, 0xfe, 0, 0, 0, 0]
        );
    }

    #[test]
    fn builds_documented_preferences_packet() {
        assert_eq!(
            preferences_packet(&config()),
            [
                0x05, 0x0f, 0x01, 0, 0x03, 0xc8, 0, 0xff, 0, 0x0c, 0x02, 0x01, 0xd8, 0, 0
            ]
        );
    }

    #[test]
    fn encodes_documented_led_modes_without_overwriting_preferences() {
        let mut config = config();
        for (mode, value) in [
            (LedMode::Breathing, 0x20),
            (LedMode::Neon, 0x30),
            (LedMode::ColorBreathing, 0x40),
        ] {
            config.led_mode = mode;
            let packet = preferences_packet(&config);
            assert_eq!(packet[3], value);
            assert_eq!(packet[9], 12);
            assert_eq!(packet[10], 2);
            assert_eq!(
                packet[12],
                packet[3..=10]
                    .iter()
                    .fold(0_u8, |sum, byte| sum.wrapping_add(*byte))
            );
        }
    }

    #[test]
    fn builds_documented_dpi_packet() {
        let packet = dpi_packet(&config()).unwrap();
        assert_eq!(packet.len(), 56);
        assert_eq!(
            &packet[..14],
            &[
                0x04, 0x38, 0x01, 0, 0, 0x3f, 0, 0, 0x12, 0x25, 0x4b, 0x5e, 0x75, 0x8d
            ]
        );
        assert_eq!(&packet[16..25], &[0, 0, 0, 0, 0, 1, 0, 0, 3]);
        assert_eq!(
            &packet[25..],
            &[
                0xff, 0, 0, 0, 0xff, 0, 0, 0xff, 0xff, 0xff, 0, 0, 0, 0xff, 0xff, 0x40, 0, 0xff,
                0xff, 0x40, 0, 0xff, 0xff, 0xff, 0, 0x0e, 0x99, 0, 0, 0, 0
            ]
        );
    }

    #[test]
    fn sets_high_dpi_masks_and_flags() {
        let mut config = config();
        config.dpis = [50, 10_100, 12_100, 18_100, 20_100, 22_000];
        config.active_dpi = 6;
        config.angle_snap = true;
        config.ripple_control = true;
        let packet = dpi_packet(&config).unwrap();
        assert_eq!(
            &packet[3..14],
            &[1, 1, 0x3f, 0x3c, 0x3c, 1, 0x76, 0x8e, 0xd4, 0xeb, 0x81]
        );
        assert_eq!(&packet[16..22], &[0, 1, 0, 0, 1, 1]);
        assert_eq!(packet[24], 6);
    }

    #[test]
    fn encodes_global_color_speed_and_stage_colors() {
        let mut config = config();
        config.led_mode = LedMode::Static;
        config.led_color = [0x12, 0x80, 0xff];
        config.led_speed = 5;
        config.dpi_colors[2] = [1, 2, 3];

        let preferences = preferences_packet(&config);
        assert_eq!(preferences[3], 0x10);
        assert_eq!(preferences[4] & 0x0f, 1);
        assert_eq!(&preferences[6..9], &[0x12, 0x80, 0xff]);
        assert_eq!(preferences[11], 2);

        let dpis = dpi_packet(&config).unwrap();
        assert_eq!(&dpis[31..34], &[1, 2, 3]);
        assert_eq!(dpis[49], 0);
    }

    #[test]
    fn manual_model_selection_allows_shared_adapter_revisions() {
        assert!(supports_x11_adapter(X11_WIRELESS_VERSION, false));
        assert!(supports_x11_adapter(0x1200, true));
        assert!(!supports_x11_adapter(0x1200, false));
        assert!(supports_x11_adapter(0x1105, true));
        assert!(!supports_x11_adapter(0x1105, false));
    }
}
