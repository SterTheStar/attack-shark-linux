use std::{fs, path::PathBuf};

use attack_shark::{Config, DeviceModel, DeviceService, X11Device};
use serde::Serialize;

#[derive(Serialize)]
struct DeviceStatus {
    model: &'static str,
    mode: &'static str,
    battery_charge: Option<u8>,
    udev_rule: UdevRuleStatus,
}

#[derive(Serialize)]
struct UdevRuleStatus {
    installed: bool,
    rule_name: &'static str,
    command: String,
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
fn apply_config(config: Config, model_override: Option<String>) -> Result<ApplyResult, String> {
    validate_shared_config(&config)?;
    let requested_model = selected_model(model_override.as_deref())?;
    let detected = match requested_model {
        Some(model) => DeviceService::detect_model(model),
        None => DeviceService::detect(),
    }
    .map_err(|error| error.to_string())?
    .ok_or("no supported Attack Shark mouse was found")?;
    let mut skipped = Vec::new();

    let model = resolve_model(detected.model, model_override.as_deref())?;
    match model {
        DeviceModel::R1 => {
            config
                .dpis
                .iter()
                .try_for_each(|dpi| Config::validate_dpi(*dpi))
                .map_err(|error| error.to_string())?;
            let mut mouse = DeviceService::open_r1(model_override.as_deref() == Some("r1"))
                .map_err(|error| error.to_string())?;
            apply_r1_config(&mut mouse, &config, &mut skipped);
        }
        DeviceModel::X11 => {
            let mut mouse = X11Device::open(model_override.as_deref() == Some("x11"))
                .map_err(|error| error.to_string())?;
            apply_x11_config(&mut mouse, &config, &mut skipped);
        }
        DeviceModel::UnknownAdapter => {
            return Err("the wireless adapter model is ambiguous; connect the mouse by cable once to identify it".into());
        }
    }

    Ok(ApplyResult { skipped })
}

fn resolve_model(detected: DeviceModel, model_override: Option<&str>) -> Result<DeviceModel, String> {
    match model_override {
        None | Some("auto") => match detected {
            DeviceModel::UnknownAdapter => Err(
                "the wireless adapter model is ambiguous; select R1 or X11 in Settings, or connect the mouse by cable once".into(),
            ),
            model => Ok(model),
        },
        Some("r1") => Ok(DeviceModel::R1),
        Some("x11") => Ok(DeviceModel::X11),
        Some(_) => Err("unsupported mouse model selection".into()),
    }
}

fn validate_shared_config(config: &Config) -> Result<(), String> {
    if !(1..=6).contains(&config.active_dpi) {
        return Err("active DPI must be between 1 and 6".into());
    }
    if !(0.5..=30.0).contains(&config.sleep_time) {
        return Err("sleep time must be between 0.5 and 30 minutes".into());
    }
    if !(1..=60).contains(&config.deep_sleep_time) {
        return Err("deep sleep time must be between 1 and 60 minutes".into());
    }
    if !(4..=50).contains(&config.key_response_time) || !config.key_response_time.is_multiple_of(2)
    {
        return Err("key response time must be an even value between 4 and 50 ms".into());
    }
    Ok(())
}

fn apply_r1_config(
    mouse: &mut attack_shark::MouseDevice,
    config: &Config,
    skipped: &mut Vec<String>,
) {
    if let Err(error) = mouse.set_polling_rate(config.polling_rate) {
        eprintln!("R1 polling rate failed: {error:?}");
        skipped.push(format!("polling rate: {error}"));
    }
    if let Err(error) = mouse.set_times(
        config.sleep_time,
        config.deep_sleep_time,
        config.key_response_time,
    ) {
        eprintln!("R1 sleep timers failed: {error:?}");
        skipped.push(format!("sleep timers: {error}"));
    }
    if let Err(error) = mouse.set_dpis(
        config.dpis,
        config.active_dpi,
        config.ripple_control,
        config.angle_snap,
    ) {
        eprintln!("R1 DPI options failed: {error:?}");
        skipped.push(format!("DPI options: {error}"));
    }
}

fn apply_x11_config(mouse: &mut X11Device, config: &Config, skipped: &mut Vec<String>) {
    if let Err(error) = mouse.set_polling_rate(config.polling_rate) {
        eprintln!("X11 polling rate failed: {error:?}");
        skipped.push(format!("polling rate: {error}"));
    }
    if let Err(error) = mouse.set_preferences(config) {
        eprintln!("X11 preferences failed: {error:?}");
        skipped.push(format!("preferences: {error}"));
    }
    if let Err(error) = mouse.set_dpis(config) {
        eprintln!("X11 DPI options failed: {error:?}");
        skipped.push(format!("DPI options: {error}"));
    }
}

#[tauri::command]
fn battery_charge() -> Result<u8, String> {
    DeviceService::open()
        .and_then(|mut mouse| mouse.battery_charge())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn device_status(model_override: Option<String>) -> Result<DeviceStatus, String> {
    let selected_model = selected_model(model_override.as_deref())?;
    let detected = match selected_model {
        Some(model) => DeviceService::detect_model(model).map_err(|error| error.to_string())?,
        None => DeviceService::detect().map_err(|error| error.to_string())?,
    };
    let model = detected
        .map(|detected| detected.model)
        .or(selected_model)
        .ok_or("no supported Attack Shark mouse was found")?;
    let model_name = match model {
        DeviceModel::R1 => "r1",
        DeviceModel::X11 => "x11",
        DeviceModel::UnknownAdapter => "adapter",
    };
    let mode = detected.map_or("unknown", |detected| match detected.connection {
        attack_shark::ConnectionMode::Wired => "wired",
        attack_shark::ConnectionMode::Wireless => "wireless",
    });
    DeviceService::start_battery_monitor();
    let battery_charge = detected.and_then(DeviceService::monitored_battery);
    Ok(DeviceStatus {
        model: model_name,
        mode,
        battery_charge,
        udev_rule: udev_rule_status(model, detected.map(|detected| detected.connection)),
    })
}

fn selected_model(model_override: Option<&str>) -> Result<Option<DeviceModel>, String> {
    match model_override {
        None | Some("auto") => Ok(None),
        Some("r1") => Ok(Some(DeviceModel::R1)),
        Some("x11") => Ok(Some(DeviceModel::X11)),
        Some(_) => Err("unsupported mouse model selection".into()),
    }
}

fn udev_rule_status(
    model: DeviceModel,
    connection: Option<attack_shark::ConnectionMode>,
) -> UdevRuleStatus {
    let (rule_name, product_id) = match model {
        DeviceModel::R1 => ("99-attack-shark-r1.rules", connection.map(|mode| if mode == attack_shark::ConnectionMode::Wired { "fa61" } else { "fa60" })),
        DeviceModel::X11 => ("99-attack-shark-x11.rules", connection.map(|mode| if mode == attack_shark::ConnectionMode::Wired { "fa55" } else { "fa60" })),
        DeviceModel::UnknownAdapter => ("99-attack-shark-x11.rules", Some("fa60")),
    };
    let command = format!(
        "sudo install -Dm644 {rule_name} /etc/udev/rules.d/{rule_name} && sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=usb --attr-match=idVendor=1d57 && sudo udevadm trigger --subsystem-match=hidraw"
    );
    let installed = [
        "/etc/udev/rules.d",
        "/run/udev/rules.d",
        "/usr/lib/udev/rules.d",
        "/lib/udev/rules.d",
    ]
    .iter()
    .filter_map(|directory| fs::read_dir(directory).ok())
    .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
    .filter(|entry| entry.path().extension().is_some_and(|extension| extension == "rules"))
    .any(|entry| {
        let Ok(contents) = fs::read_to_string(entry.path()) else {
            return false;
        };
        match product_id {
            Some(product_id) => has_hidraw_rule(&contents, product_id),
            None => entry.file_name() == rule_name && contents.contains("KERNEL==\"hidraw*\""),
        }
    });
    UdevRuleStatus {
        installed,
        rule_name,
        command,
    }
}

fn has_hidraw_rule(contents: &str, product_id: &str) -> bool {
    contents.contains("KERNEL==\"hidraw*\"")
        && contents.contains("ATTRS{idVendor}==\"1d57\"")
        && contents.contains(&format!("ATTRS{{idProduct}}==\"{product_id}\""))
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
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            load_config,
            apply_config,
            battery_charge,
            device_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running Attack Shark");
}
