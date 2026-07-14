use std::{
    sync::{Mutex, OnceLock},
    time::Duration,
};

use hidapi::{DeviceInfo, HidApi, HidDevice};

use crate::{
    config::Config,
    error::{DriverError, Result},
    protocol,
};

const VENDOR_ID: u16 = 0x1d57;
const WIRELESS_PRODUCT_ID: u16 = 0xfa60;
const WIRED_PRODUCT_ID: u16 = 0xfa61;
const CONFIGURATION_INTERFACE: i32 = 2;
const R1_WIRELESS_VERSION: u16 = 0x1105;
const X11_WIRELESS_VERSION: u16 = 0x1108;
const READ_TIMEOUT: Duration = Duration::from_secs(1);
const WIRED_PACKET_DELAY: Duration = Duration::from_millis(300);
const MONITOR_RETRY_DELAY: Duration = Duration::from_secs(1);

static BATTERY_MONITOR_STARTED: OnceLock<()> = OnceLock::new();
static BATTERY_STATUS: OnceLock<Mutex<Option<BatteryStatus>>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceModel {
    R1,
    X11,
    UnknownAdapter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionMode {
    Wired,
    Wireless,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DetectedDevice {
    pub model: DeviceModel,
    pub connection: ConnectionMode,
}

#[derive(Clone, Copy)]
struct BatteryStatus {
    model: DeviceModel,
    charge: u8,
}

pub struct MouseDevice {
    device: HidDevice,
    wired: bool,
}

pub struct DeviceService;

impl DeviceService {
    pub fn open() -> Result<MouseDevice> {
        MouseDevice::open(false)
    }

    pub fn open_r1(allow_unknown_adapter: bool) -> Result<MouseDevice> {
        MouseDevice::open(allow_unknown_adapter)
    }

    pub fn detect() -> Result<Option<DetectedDevice>> {
        let api = HidApi::new()?;
        let mut adapter = None;

        for device in configuration_devices(&api) {
            let Some(detected) = classify_device(device.product_id(), device.release_number())
            else {
                continue;
            };
            if detected.connection == ConnectionMode::Wired {
                return Ok(Some(detected));
            }
            adapter = Some(detected.model);
        }

        Ok(adapter.map(|model| DetectedDevice {
            model,
            connection: ConnectionMode::Wireless,
        }))
    }

    pub fn detect_model(model: DeviceModel) -> Result<Option<DetectedDevice>> {
        let api = HidApi::new()?;
        let mut wireless_match = None;
        let mut unknown_adapter = false;

        for device in configuration_devices(&api) {
            let Some(detected) = classify_device(device.product_id(), device.release_number())
            else {
                continue;
            };
            if detected.model == model {
                if detected.connection == ConnectionMode::Wired {
                    return Ok(Some(detected));
                }
                wireless_match = Some(detected);
            } else if detected.model == DeviceModel::UnknownAdapter {
                unknown_adapter = true;
            }
        }

        Ok(wireless_match.or_else(|| {
            unknown_adapter.then_some(DetectedDevice {
                model,
                connection: ConnectionMode::Wireless,
            })
        }))
    }

    pub fn start_battery_monitor() {
        BATTERY_MONITOR_STARTED.get_or_init(|| {
            BATTERY_STATUS.get_or_init(|| Mutex::new(None));
            std::thread::Builder::new()
                .name("attack-shark-battery".into())
                .spawn(monitor_battery_reports)
                .expect("battery monitor thread must start");
        });
    }

    pub fn monitored_battery(detected: DetectedDevice) -> Option<u8> {
        if detected.connection != ConnectionMode::Wireless {
            return None;
        }
        BATTERY_STATUS
            .get()
            .and_then(|status| status.lock().ok())
            .and_then(|status| status.as_ref().copied())
            .and_then(|status| (status.model == detected.model).then_some(status.charge))
    }
}

fn monitor_battery_reports() {
    loop {
        let Ok(api) = HidApi::new() else {
            std::thread::sleep(MONITOR_RETRY_DELAY);
            continue;
        };
        let Some(device_info) = configuration_devices(&api).find(|device| {
            device.product_id() == WIRELESS_PRODUCT_ID
                && matches!(
                    classify_device(device.product_id(), device.release_number()),
                    Some(DetectedDevice {
                        model: DeviceModel::R1 | DeviceModel::X11,
                        ..
                    })
                )
        }) else {
            clear_battery_status();
            std::thread::sleep(MONITOR_RETRY_DELAY);
            continue;
        };
        let Some(detected) =
            classify_device(device_info.product_id(), device_info.release_number())
        else {
            continue;
        };
        let Ok(device) = device_info.open_device(&api) else {
            std::thread::sleep(MONITOR_RETRY_DELAY);
            continue;
        };
        let mut report = [0; 64];
        while let Ok(transferred) =
            device.read_timeout(&mut report, READ_TIMEOUT.as_millis() as i32)
        {
            if let Some(charge) = battery_charge_from_report(detected.model, &report[..transferred])
                && let Some(mut status) = BATTERY_STATUS.get().and_then(|status| status.lock().ok())
            {
                *status = Some(BatteryStatus {
                    model: detected.model,
                    charge,
                });
            }
        }
        clear_battery_status();
        std::thread::sleep(MONITOR_RETRY_DELAY);
    }
}

fn clear_battery_status() {
    if let Some(mut status) = BATTERY_STATUS.get().and_then(|status| status.lock().ok()) {
        *status = None;
    }
}

fn battery_charge_from_report(model: DeviceModel, report: &[u8]) -> Option<u8> {
    if report.len() < 5 {
        return None;
    }
    match model {
        DeviceModel::R1 if report[4] <= 10 => Some(report[4] * 10),
        DeviceModel::X11 if report.starts_with(&[0x03, 0x55, 0x40, 0x01]) && report[4] <= 100 => {
            Some(report[4])
        }
        _ => None,
    }
}

impl MouseDevice {
    pub fn open(allow_unknown_adapter: bool) -> Result<Self> {
        let api = HidApi::new()?;
        for product_id in [WIRED_PRODUCT_ID, WIRELESS_PRODUCT_ID] {
            for device in configuration_devices(&api) {
                if device.product_id() != product_id {
                    continue;
                }
                if product_id == WIRELESS_PRODUCT_ID
                    && !supports_r1_adapter(device.release_number(), allow_unknown_adapter)
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

    pub fn apply_config(&mut self, config: &Config) -> Result<()> {
        self.set_polling_rate(config.polling_rate)
            .map_err(|source| DriverError::Operation {
                operation: "setting polling rate",
                source: Box::new(source),
            })?;
        self.set_times(
            config.sleep_time,
            config.deep_sleep_time,
            config.key_response_time,
        )
        .map_err(|source| DriverError::Operation {
            operation: "setting sleep timers",
            source: Box::new(source),
        })?;
        self.set_dpis(
            config.dpis,
            config.active_dpi,
            config.ripple_control,
            config.angle_snap,
        )
        .map_err(|source| DriverError::Operation {
            operation: "setting DPI options",
            source: Box::new(source),
        })
    }

    pub const fn is_wired(&self) -> bool {
        self.wired
    }

    pub fn battery_charge(&mut self) -> Result<u8> {
        if self.wired {
            return Ok(0);
        }
        self.read_status()
    }

    pub fn read_status(&mut self) -> Result<u8> {
        let mut buffer = [0; 64];
        let transferred = self
            .device
            .read_timeout(&mut buffer, READ_TIMEOUT.as_millis() as i32)?;
        if transferred < 5 {
            return Err(DriverError::InvalidReport(format!(
                "expected at least 5 status bytes, received {transferred}"
            )));
        }
        Ok(buffer[4] * 10)
    }

    pub fn set_polling_rate(&mut self, rate: crate::config::PollingRate) -> Result<()> {
        self.send(&protocol::polling_rate_packet(rate))
    }

    pub fn set_times(&mut self, sleep_time: f64, deep_sleep: u8, key_response: u8) -> Result<()> {
        self.send(&protocol::times_packet(
            sleep_time,
            deep_sleep,
            key_response,
        ))
    }

    pub fn set_dpis(
        &mut self,
        dpis: [u16; 6],
        active_dpi: u8,
        ripple_control: bool,
        angle_snap: bool,
    ) -> Result<()> {
        self.send(&protocol::dpi_packet(
            dpis,
            active_dpi,
            ripple_control,
            angle_snap,
        ))
    }

    fn send(&mut self, packet: &[u8]) -> Result<()> {
        for _ in 0..3 {
            self.device.send_feature_report(packet)?;
            if self.wired {
                std::thread::sleep(WIRED_PACKET_DELAY);
                return Ok(());
            }
            let mut acknowledgment = [0; 64];
            let transferred = self
                .device
                .read_timeout(&mut acknowledgment, READ_TIMEOUT.as_millis() as i32)?;
            if transferred == 0 || (transferred >= 3 && acknowledgment[2] == 0x50) {
                return Ok(());
            }
        }
        Err(DriverError::InvalidReport(
            "receiver did not return a command acknowledgement".into(),
        ))
    }
}

pub(crate) fn configuration_devices(api: &HidApi) -> impl Iterator<Item = &DeviceInfo> {
    api.device_list().filter(|device| {
        device.vendor_id() == VENDOR_ID && device.interface_number() == CONFIGURATION_INTERFACE
    })
}

fn classify_device(product_id: u16, release_number: u16) -> Option<DetectedDevice> {
    match product_id {
        0xfa55 => Some(DetectedDevice {
            model: DeviceModel::X11,
            connection: ConnectionMode::Wired,
        }),
        WIRED_PRODUCT_ID => Some(DetectedDevice {
            model: DeviceModel::R1,
            connection: ConnectionMode::Wired,
        }),
        WIRELESS_PRODUCT_ID => Some(DetectedDevice {
            model: match release_number {
                X11_WIRELESS_VERSION => DeviceModel::X11,
                R1_WIRELESS_VERSION => DeviceModel::R1,
                _ => DeviceModel::UnknownAdapter,
            },
            connection: ConnectionMode::Wireless,
        }),
        _ => None,
    }
}

fn supports_r1_adapter(release_number: u16, allow_unknown_adapter: bool) -> bool {
    release_number != X11_WIRELESS_VERSION
        && (allow_unknown_adapter || release_number == R1_WIRELESS_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_wired_models_and_documented_adapter_revisions() {
        assert_eq!(
            classify_device(0xfa55, 0x0102).unwrap().model,
            DeviceModel::X11
        );
        assert_eq!(
            classify_device(0xfa61, 0x0103).unwrap().model,
            DeviceModel::R1
        );
        assert_eq!(
            classify_device(0xfa60, 0x1108).unwrap().model,
            DeviceModel::X11
        );
        assert_eq!(
            classify_device(0xfa60, 0x1105).unwrap().model,
            DeviceModel::R1
        );
        assert_eq!(
            classify_device(0xfa60, 0x1100).unwrap().model,
            DeviceModel::UnknownAdapter
        );
    }

    #[test]
    fn never_opens_a_known_x11_adapter_as_r1() {
        assert!(supports_r1_adapter(R1_WIRELESS_VERSION, false));
        assert!(supports_r1_adapter(0x1200, true));
        assert!(!supports_r1_adapter(0x1200, false));
        assert!(!supports_r1_adapter(X11_WIRELESS_VERSION, true));
    }

    #[test]
    fn decodes_model_specific_battery_reports() {
        assert_eq!(
            battery_charge_from_report(DeviceModel::R1, &[3, 0, 0, 0, 8]),
            Some(80)
        );
        assert_eq!(
            battery_charge_from_report(DeviceModel::X11, &[3, 0x55, 0x40, 1, 73]),
            Some(73)
        );
        assert_eq!(
            battery_charge_from_report(DeviceModel::X11, &[3, 0, 0, 0, 73]),
            None
        );
    }
}
