//! Reusable driver core. UI frontends should depend on this crate rather than
//! talking to libusb or constructing protocol packets themselves.

pub mod config;
pub mod device;
pub mod error;
pub mod protocol;

pub use config::{Config, PollingRate};
pub use device::{DeviceService, MouseDevice};
pub use error::{DriverError, Result};
