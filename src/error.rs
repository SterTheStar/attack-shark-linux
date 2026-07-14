use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, DriverError>;

#[derive(Debug, Error)]
pub enum DriverError {
    #[error("{operation}: {source}")]
    Operation {
        operation: &'static str,
        #[source]
        source: Box<DriverError>,
    },
    #[error("could not load configuration from {path}: {source}")]
    ConfigIo {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("invalid mouse report: {0}")]
    InvalidReport(String),
    #[error("no supported Attack Shark mouse was found")]
    DeviceNotFound,
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),
}
