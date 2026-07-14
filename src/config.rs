use std::{fs, io, path::Path};

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

    const fn hz(self) -> u16 {
        match self {
            Self::Hz125 => 125,
            Self::Hz250 => 250,
            Self::Hz500 => 500,
            Self::Hz1000 => 1000,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum LedMode {
    #[default]
    Disabled,
    Static,
    Breathing,
    Neon,
    ColorBreathing,
    StaticDpi,
    BreathingDpi,
}

const fn default_led_color() -> [u8; 3] {
    [0, 255, 0]
}

const fn default_stage_colors() -> [[u8; 3]; 6] {
    [
        [255, 0, 0],
        [0, 255, 0],
        [0, 255, 255],
        [255, 0, 0],
        [0, 255, 255],
        [64, 0, 255],
    ]
}

const fn default_led_speed() -> u8 {
    3
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
    #[serde(default)]
    pub led_mode: LedMode,
    #[serde(default = "default_led_color")]
    pub led_color: [u8; 3],
    #[serde(default = "default_stage_colors")]
    pub dpi_colors: [[u8; 3]; 6],
    #[serde(default = "default_led_speed")]
    pub led_speed: u8,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|source| DriverError::ConfigIo {
                        path: path.to_path_buf(),
                        source,
                    })?;
                }
                fs::write(path, include_str!("../config.toml")).map_err(|source| {
                    DriverError::ConfigIo {
                        path: path.to_path_buf(),
                        source,
                    }
                })?;
                include_str!("../config.toml").to_owned()
            }
            Err(source) => {
                return Err(DriverError::ConfigIo {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };
        Self::parse(&contents)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| DriverError::ConfigIo {
                path: path.to_path_buf(),
                source,
            })?;
        }
        let contents = format!(
            "# Attack Shark mouse configuration.\n\npolling_rate = {}\ndpis = {:?}\nactive_dpi = {}\nsleep_time = {}\ndeep_sleep_time = {}\nkey_response_time = {}\nripple_control = {}\nangle_snap = {}\nled_mode = \"{:?}\"\nled_color = {:?}\ndpi_colors = {:?}\nled_speed = {}\n",
            self.polling_rate.hz(),
            self.dpis,
            self.active_dpi,
            self.sleep_time,
            self.deep_sleep_time,
            self.key_response_time,
            self.ripple_control,
            self.angle_snap,
            self.led_mode,
            self.led_color,
            self.dpi_colors,
            self.led_speed,
        );
        let temporary = path.with_extension("toml.tmp");
        fs::write(&temporary, contents).map_err(|source| DriverError::ConfigIo {
            path: temporary.clone(),
            source,
        })?;
        fs::rename(&temporary, path).map_err(|source| DriverError::ConfigIo {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn parse(contents: &str) -> Result<Self> {
        let values: TomlConfig = toml::from_str(contents)
            .map_err(|error| DriverError::InvalidConfig(error.to_string()))?;
        let polling_rate = PollingRate::from_hz(values.polling_rate)?;
        let dpis = values.dpis;
        dpis.iter()
            .try_for_each(|&dpi| Self::validate_any_dpi(dpi))?;
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
        if !(1..=5).contains(&values.led_speed) {
            return Err(DriverError::InvalidConfig(
                "led_speed must be between 1 and 5".into(),
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
            led_mode: values.led_mode,
            led_color: values.led_color,
            dpi_colors: values.dpi_colors,
            led_speed: values.led_speed,
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

    pub fn validate_x11_dpi(value: u16) -> Result<()> {
        if value == 50
            || ((100..=10_000).contains(&value) && value.is_multiple_of(50))
            || ((10_100..=22_000).contains(&value) && value.is_multiple_of(100))
        {
            return Ok(());
        }
        Err(DriverError::InvalidConfig(format!(
            "X11 DPI must use 50-step values up to 10000 or 100-step values up to 22000; got {value}"
        )))
    }

    fn validate_any_dpi(value: u16) -> Result<()> {
        Self::validate_dpi(value).or_else(|_| Self::validate_x11_dpi(value))
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
    #[serde(default)]
    led_mode: LedMode,
    #[serde(default = "default_led_color")]
    led_color: [u8; 3],
    #[serde(default = "default_stage_colors")]
    dpi_colors: [[u8; 3]; 6],
    #[serde(default = "default_led_speed")]
    led_speed: u8,
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

    #[test]
    fn creates_missing_configuration_from_the_default_template() {
        let path =
            std::env::temp_dir().join(format!("attack-shark-config-test-{}", std::process::id()));
        let config_path = path.join("attack-shark/config.toml");
        let config = Config::load(&config_path).unwrap();

        assert_eq!(config.dpis, [800, 1600, 3200, 4000, 5000, 12000]);
        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            include_str!("../config.toml")
        );
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn saves_and_loads_applied_settings() {
        let path =
            std::env::temp_dir().join(format!("attack-shark-save-test-{}", std::process::id()));
        let config_path = path.join("config.toml");
        let mut config = Config::parse(include_str!("../config.toml")).unwrap();
        config.polling_rate = PollingRate::Hz1000;
        config.led_mode = LedMode::Neon;

        config.save(&config_path).unwrap();
        let loaded = Config::load(&config_path).unwrap();

        assert_eq!(loaded.polling_rate, PollingRate::Hz1000);
        assert_eq!(loaded.led_mode, LedMode::Neon);
        fs::remove_dir_all(path).unwrap();
    }
}
