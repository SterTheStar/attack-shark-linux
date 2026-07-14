use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::error::{DriverError, Result};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PollingRate {
    Hz125,
    Hz250,
    Hz500,
    Hz1000,
}

impl PollingRate {
    pub fn from_hz(value: u16) -> Result<Self> {
        match value {
            125 => Ok(Self::Hz125),
            250 => Ok(Self::Hz250),
            500 => Ok(Self::Hz500),
            1000 => Ok(Self::Hz1000),
            _ => Err(DriverError::InvalidConfig(format!(
                "polling_rate must be 125, 250, 500, or 1000; got {value}"
            ))),
        }
    }

    pub(crate) const fn protocol_value(self) -> u16 {
        match self {
            Self::Hz125 => 0xf708,
            Self::Hz250 => 0xfb04,
            Self::Hz500 => 0xfd02,
            Self::Hz1000 => 0xfe01,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub polling_rate: PollingRate,
    pub dpis: [u16; 6],
    /// One-based profile index, as used by the mouse protocol.
    pub active_dpi: u8,
    pub sleep_time: f64,
    pub deep_sleep_time: u8,
    pub key_response_time: u8,
    pub angle_snap: bool,
    pub ripple_control: bool,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|source| DriverError::ConfigIo {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&contents)
    }

    pub fn parse(contents: &str) -> Result<Self> {
        let values: TomlConfig = toml::from_str(contents)
            .map_err(|error| DriverError::InvalidConfig(error.to_string()))?;
        let polling_rate = PollingRate::from_hz(values.polling_rate)?;
        let dpis = values.dpis;
        dpis.iter().try_for_each(|&dpi| Self::validate_dpi(dpi))?;
        let active_dpi = values.active_dpi;
        if !(1..=6).contains(&active_dpi) {
            return Err(DriverError::InvalidConfig(
                "active_dpi must be between 1 and 6".into(),
            ));
        }
        let sleep_time = values.sleep_time;
        if !(0.5..=30.0).contains(&sleep_time) {
            return Err(DriverError::InvalidConfig(
                "sleep_time must be between 0.5 and 30".into(),
            ));
        }
        let deep_sleep_time = values.deep_sleep_time;
        if !(1..=60).contains(&deep_sleep_time) {
            return Err(DriverError::InvalidConfig(
                "deep_sleep_time must be between 1 and 60".into(),
            ));
        }
        let key_response_time = values.key_response_time;
        if !(4..=50).contains(&key_response_time) || !key_response_time.is_multiple_of(2) {
            return Err(DriverError::InvalidConfig(
                "key_response_time must be an even value between 4 and 50".into(),
            ));
        }
        Ok(Self {
            polling_rate,
            dpis,
            active_dpi,
            sleep_time,
            deep_sleep_time,
            key_response_time,
            angle_snap: values.angle_snap,
            ripple_control: values.ripple_control,
        })
    }

    pub fn validate_dpi(value: u16) -> Result<()> {
        if !(100..=18_000).contains(&value) || !value.is_multiple_of(100) {
            return Err(DriverError::InvalidConfig(format!(
                "DPI must be a multiple of 100 between 100 and 18000; got {value}"
            )));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct TomlConfig {
    polling_rate: u16,
    dpis: [u16; 6],
    active_dpi: u8,
    sleep_time: f64,
    deep_sleep_time: u8,
    key_response_time: u8,
    angle_snap: bool,
    ripple_control: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_complete_configuration() {
        let config = Config::parse(include_str!("../config.toml")).unwrap();
        assert_eq!(config.dpis, [800, 1600, 3200, 4000, 5000, 12000]);
        assert_eq!(config.active_dpi, 3);
    }

    #[test]
    fn rejects_incomplete_configuration() {
        let config = "polling_rate = 1000\n";
        assert!(Config::parse(config).is_err());
    }
}
