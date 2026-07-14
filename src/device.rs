use std::time::Duration;

use rusb::{Context, DeviceHandle, UsbContext};

use crate::{
    config::Config,
    error::{DriverError, Result},
    protocol::{self, Transport},
};

const VENDOR_ID: u16 = 0x1d57;
const WIRELESS_PRODUCT_ID: u16 = 0xfa60;
const WIRED_PRODUCT_ID: u16 = 0xfa61;
const USB_TIMEOUT: Duration = Duration::from_secs(1);

pub struct MouseDevice {
    handle: DeviceHandle<Context>,
    wired: bool,
    kernel_driver_detached: bool,
}

pub struct DeviceService;

impl DeviceService {
    pub fn open() -> Result<MouseDevice> {
        MouseDevice::open()
    }
}

impl MouseDevice {
    pub fn open() -> Result<Self> {
        let context = Context::new()?;
        let devices = context.devices()?;
        // Prefer the cable whenever it is available.
        for product_id in [WIRED_PRODUCT_ID, WIRELESS_PRODUCT_ID] {
            for device in devices.iter() {
                let Ok(descriptor) = device.device_descriptor() else {
                    continue;
                };
                if descriptor.vendor_id() != VENDOR_ID || descriptor.product_id() != product_id {
                    continue;
                }
                let wired = product_id == WIRED_PRODUCT_ID;
                let handle = device.open()?;
                let kernel_driver_detached = handle
                    .kernel_driver_active(protocol::INTERFACE)
                    .unwrap_or(false);
                if kernel_driver_detached {
                    handle.detach_kernel_driver(protocol::INTERFACE)?;
                }
                if let Err(error) = handle.claim_interface(protocol::INTERFACE) {
                    if kernel_driver_detached {
                        let _ = handle.attach_kernel_driver(protocol::INTERFACE);
                    }
                    return Err(error.into());
                }
                return Ok(Self {
                    handle,
                    wired,
                    kernel_driver_detached,
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

    /// Reads the wireless receiver's unsolicited status report.
    ///
    /// The original driver always consumed this report before issuing a
    /// command, otherwise it can be mistaken for a command acknowledgment.
    pub fn read_status(&mut self) -> Result<u8> {
        // Interface 2 advertises 64-byte interrupt reports. The Odin version
        // requested 64 bytes but allocated only 5, causing a buffer overflow.
        let mut buffer = [0; 64];
        let transferred =
            self.handle
                .read_interrupt(protocol::ACK_ENDPOINT, &mut buffer, USB_TIMEOUT)?;
        if transferred < 5 {
            return Err(DriverError::InvalidReport(format!(
                "expected at least 5 status bytes, received {transferred}"
            )));
        }
        Ok(buffer[4] * 10)
    }

    pub fn set_polling_rate(&mut self, rate: crate::config::PollingRate) -> Result<()> {
        let packet = protocol::polling_rate_packet(rate);
        self.send(0x306, &packet)
    }

    pub fn set_times(&mut self, sleep_time: f64, deep_sleep: u8, key_response: u8) -> Result<()> {
        let packet = protocol::times_packet(sleep_time, deep_sleep, key_response);
        self.send(0x305, &packet)
    }

    pub fn set_dpis(
        &mut self,
        dpis: [u16; 6],
        active_dpi: u8,
        ripple_control: bool,
        angle_snap: bool,
    ) -> Result<()> {
        let packet = protocol::dpi_packet(dpis, active_dpi, ripple_control, angle_snap);
        self.send(0x304, &packet)
    }
}

impl Transport for MouseDevice {
    fn send(&mut self, value: u16, packet: &[u8]) -> Result<()> {
        for _ in 0..3 {
            self.handle.write_control(
                protocol::CONTROL_REQUEST_TYPE,
                protocol::CONTROL_REQUEST,
                value,
                protocol::INTERFACE as u16,
                packet,
                USB_TIMEOUT,
            )?;
            if self.wired {
                return Ok(());
            }
            let mut acknowledgment = [0; 5];
            match self.handle.read_interrupt(
                protocol::ACK_ENDPOINT,
                &mut acknowledgment,
                USB_TIMEOUT,
            ) {
                Ok(_) if acknowledgment[2] == 0x50 => return Ok(()),
                // Some receivers accept the control report but do not
                // produce the optional interrupt acknowledgement.
                Err(rusb::Error::Timeout) => return Ok(()),
                Err(error) => return Err(error.into()),
                Ok(_) => {}
            }
        }
        Err(rusb::Error::Timeout.into())
    }
}

impl Drop for MouseDevice {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(protocol::INTERFACE);
        if self.kernel_driver_detached {
            let _ = self.handle.attach_kernel_driver(protocol::INTERFACE);
        }
    }
}
