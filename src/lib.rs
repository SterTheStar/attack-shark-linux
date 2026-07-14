//! Reusable driver core. UI frontends should depend on this crate rather than
//! talking to libusb or constructing protocol packets themselves.

pub mod config;
pub mod device;
pub mod error;
pub mod protocol;
pub mod x11;

pub use config::{Config, PollingRate};
pub use device::{ConnectionMode, DetectedDevice, DeviceModel, DeviceService, MouseDevice};
pub use error::{DriverError, Result};
pub use x11::X11Device;
