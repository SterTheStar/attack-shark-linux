use std::path::PathBuf;

use attack_shark::{Config, DeviceService};
use serde::Serialize;

#[derive(Serialize)]
struct DeviceStatus {
    mode: &'static str,
    battery_charge: Option<u8>,
}

#[derive(Serialize)]
struct ApplyResult {
    skipped: Vec<String>,
}

#[tauri::command]
fn load_config() -> Result<Config, String> {
    Config::load(default_config_path()).map_err(|error| error.to_string())
}

#[tauri::command]
fn apply_config(config: Config) -> Result<ApplyResult, String> {
    config
        .dpis
        .iter()
        .try_for_each(|dpi| Config::validate_dpi(*dpi))
        .map_err(|error| error.to_string())?;
    if !(1..=6).contains(&config.active_dpi) {
        return Err("active DPI must be between 1 and 6".into());
    }
    let mut mouse = DeviceService::open().map_err(|error| error.to_string())?;
    let mut skipped = Vec::new();

    if let Err(error) = mouse.set_polling_rate(config.polling_rate) {
        eprintln!("setting polling rate failed: {error:?}");
        skipped.push(format!("polling rate: {error}"));
    }
    if let Err(error) = mouse.set_times(
        config.sleep_time,
        config.deep_sleep_time,
        config.key_response_time,
    ) {
        eprintln!("setting sleep timers failed: {error:?}");
        skipped.push(format!("sleep timers: {error}"));
    }
    if let Err(error) = mouse.set_dpis(
        config.dpis,
        config.active_dpi,
        config.ripple_control,
        config.angle_snap,
    ) {
        eprintln!("setting DPI options failed: {error:?}");
        skipped.push(format!("DPI options: {error}"));
    }

    Ok(ApplyResult { skipped })
}

#[tauri::command]
fn battery_charge() -> Result<u8, String> {
    DeviceService::open()
        .and_then(|mut mouse| mouse.battery_charge())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn device_status() -> Result<DeviceStatus, String> {
    let mut mouse = DeviceService::open().map_err(|error| error.to_string())?;
    let mode = if mouse.is_wired() {
        "wired"
    } else {
        "wireless"
    };
    // Battery reports are optional on some receiver firmware versions.
    let battery_charge = (!mouse.is_wired())
        .then(|| mouse.battery_charge().ok())
        .flatten();
    Ok(DeviceStatus {
        mode,
        battery_charge,
    })
}

fn default_config_path() -> PathBuf {
    let user_config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|path| path.join("attack-shark.ini"));
    if let Some(path) = user_config.filter(|path| path.exists()) {
        return path;
    }

    let system_config = PathBuf::from("/etc/attack-shark.ini");
    if system_config.exists() {
        return system_config;
    }

    // This keeps `cargo run -- --gui` aligned with the repository default.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../attack-shark.ini")
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_config,
            apply_config,
            battery_charge,
            device_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running Attack Shark");
}
