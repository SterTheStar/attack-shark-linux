use crate::{config::PollingRate, error::Result};

pub const INTERFACE: u8 = 2;
pub const ACK_ENDPOINT: u8 = 0x83;
pub const CONTROL_REQUEST_TYPE: u8 = 0x21;
pub const CONTROL_REQUEST: u8 = 0x09;

pub fn dpi_value(dpi: u16) -> u8 {
    assert!((100..=18_000).contains(&dpi) && dpi.is_multiple_of(100));
    DPI_VALUES[(dpi / 100 - 1) as usize]
}

// Exact table from dpi.odin. The device uses discrete values rather than a
// formula, notably changing encoding after 10,000 DPI.
const DPI_VALUES: [u8; 180] = [
    0x02, 0x04, 0x06, 0x09, 0x0b, 0x0e, 0x10, 0x12, 0x15, 0x17, 0x19, 0x1c, 0x1e, 0x20, 0x23, 0x25,
    0x27, 0x2a, 0x2c, 0x2f, 0x31, 0x33, 0x36, 0x38, 0x3a, 0x3d, 0x3f, 0x41, 0x44, 0x46, 0x48, 0x4b,
    0x4d, 0x4f, 0x52, 0x54, 0x57, 0x59, 0x5b, 0x5e, 0x60, 0x62, 0x65, 0x67, 0x69, 0x6c, 0x6e, 0x70,
    0x73, 0x75, 0x77, 0x7a, 0x7c, 0x7f, 0x81, 0x83, 0x86, 0x88, 0x8a, 0x8d, 0x8f, 0x91, 0x94, 0x96,
    0x98, 0x9b, 0x9d, 0x9f, 0xa2, 0xa4, 0xa7, 0xa9, 0xab, 0xae, 0xb0, 0xb2, 0xb5, 0xb7, 0xb9, 0xbc,
    0xbe, 0xc0, 0xc3, 0xc5, 0xc7, 0xca, 0xcc, 0xcf, 0xd1, 0xd3, 0xd6, 0xd8, 0xda, 0xdd, 0xdf, 0xe1,
    0xe4, 0xe6, 0xe8, 0xeb, 0x76, 0x77, 0x79, 0x7a, 0x7b, 0x7c, 0x7d, 0x7f, 0x80, 0x81, 0x82, 0x83,
    0x84, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8d, 0x8e, 0x8f, 0x90, 0x91, 0x93, 0x94, 0x95, 0x96,
    0x97, 0x98, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa7, 0xa8, 0xa9,
    0xaa, 0xab, 0xac, 0xae, 0xaf, 0xb0, 0xb1, 0xb2, 0xb3, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xbb, 0xbc,
    0xbd, 0xbe, 0xbf, 0xc0, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcc, 0xcd, 0xcf,
    0xd0, 0xd1, 0xd2, 0xd3,
];

pub fn polling_rate_packet(rate: PollingRate) -> [u8; 9] {
    let mut packet = [0x06, 0x09, 0x01, 0x01, 0, 0, 0, 0, 0];
    packet[3..5].copy_from_slice(&rate.protocol_value().to_le_bytes());
    packet
}

pub fn times_packet(sleep_time: f64, deep_sleep: u8, key_response: u8) -> [u8; 15] {
    let mut packet = [
        0x05, 0x0f, 0x01, 0, 0x03, 0x18, 0, 0, 0xff, 0x04, 0x02, 0x01, 0x20, 0, 0,
    ];
    packet[4] = 0x03 | (deep_sleep & 0xf0);
    packet[5] = 0x08 | ((deep_sleep & 0x0f) << 4);
    packet[9] = (sleep_time * 2.0) as u8;
    packet[10] = key_response / 2;
    packet[12] =
        (((deep_sleep & 0x0f) + ((deep_sleep >> 4) & 0x0f)) << 4) + 0x0a + packet[9] + packet[10];
    packet
}

pub fn dpi_packet(
    dpis: [u16; 6],
    active_dpi: u8,
    ripple_control: bool,
    angle_snap: bool,
) -> [u8; 56] {
    let mut packet: [u8; 56] = [
        0x04, 0x38, 0x01, 0, 0, 0x3f, 0, 0, 2, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        0xff, 0, 0, 0, 0xff, 0, 0, 0, 0xff, 0xff, 0xff, 0, 0, 0xff, 0xff, 0xff, 0, 0xff, 0xff,
        0x40, 0, 0xff, 0xff, 0xff, 2, 0x0d, 0x75, 0, 0, 0, 0,
    ];
    let mut checksum: u16 = 0x0d75;
    let mut above_12k = 0;
    for (index, dpi) in dpis.iter().enumerate() {
        let encoded = dpi_value(*dpi);
        packet[index + 8] = encoded;
        checksum += encoded as u16;
        let special_range = u8::from((10_100..=12_000).contains(dpi));
        packet[index + 16] = special_range;
        checksum += special_range as u16;
        if *dpi > 12_000 {
            above_12k |= 1 << index;
        }
    }
    packet[6] = above_12k;
    packet[7] = above_12k;
    checksum += (above_12k * 2) as u16;
    packet[24] = active_dpi;
    checksum += (active_dpi - 1) as u16;
    if ripple_control {
        packet[4] = 1;
        checksum += 1;
    }
    if angle_snap {
        packet[3] = 1;
        checksum += 1;
    }
    packet[50..52].copy_from_slice(&checksum.to_be_bytes());
    packet
}

pub trait Transport {
    fn send(&mut self, value: u16, packet: &[u8]) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_odin_dpi_encoding_boundaries() {
        assert_eq!(dpi_value(100), 0x02);
        assert_eq!(dpi_value(10_000), 0xeb);
        assert_eq!(dpi_value(10_100), 0x76);
        assert_eq!(dpi_value(12_000), 0x8d);
        assert_eq!(dpi_value(18_000), 0xd3);
    }

    #[test]
    fn builds_the_odin_time_packet() {
        assert_eq!(
            times_packet(6.0, 12, 4),
            [
                0x05, 0x0f, 0x01, 0, 0x03, 0xc8, 0, 0, 0xff, 12, 2, 1, 0xd8, 0, 0
            ]
        );
    }

    #[test]
    fn builds_little_endian_polling_rate_packets() {
        assert_eq!(
            polling_rate_packet(PollingRate::Hz125),
            [0x06, 0x09, 0x01, 0x08, 0xf7, 0, 0, 0, 0]
        );
        assert_eq!(
            polling_rate_packet(PollingRate::Hz250),
            [0x06, 0x09, 0x01, 0x04, 0xfb, 0, 0, 0, 0]
        );
        assert_eq!(
            polling_rate_packet(PollingRate::Hz500),
            [0x06, 0x09, 0x01, 0x02, 0xfd, 0, 0, 0, 0]
        );
        assert_eq!(
            polling_rate_packet(PollingRate::Hz1000),
            [0x06, 0x09, 0x01, 0x01, 0xfe, 0, 0, 0, 0]
        );
    }

    #[test]
    fn builds_the_odin_dpi_packet() {
        let packet = dpi_packet([800, 1600, 3200, 4000, 5000, 12000], 3, false, false);
        assert_eq!(packet.len(), 56);
        assert_eq!(&packet[6..14], &[0, 0, 0x12, 0x25, 0x4b, 0x5e, 0x75, 0x8d]);
        assert_eq!(&packet[16..22], &[0, 0, 0, 0, 0, 1]);
        assert_eq!(packet[24], 3);
        assert_eq!(&packet[50..52], &[0x0f, 0x5a]);
    }
}
