use std::{path::PathBuf, process::Command};

use attack_shark::{Config, DeviceService, DriverError, PollingRate, Result};
use clap::{CommandFactory, Parser};

#[derive(Debug, Parser)]
#[command(about = "Configure an Attack Shark mouse")]
struct Cli {
    /// Start the Tauri desktop interface.
    #[arg(long)]
    gui: bool,
    #[arg(long)]
    config_path: Option<PathBuf>,
    #[arg(long)]
    reapply_config: bool,
    #[arg(long)]
    query_charge: bool,
    #[arg(long)]
    polling_rate: Option<u16>,
    #[arg(long)]
    key_response_time: Option<u8>,
    #[arg(long)]
    sleep_time: Option<f64>,
    #[arg(long)]
    deep_sleep_time: Option<u8>,
    #[arg(long)]
    ripple_control: Option<bool>,
    #[arg(long)]
    angle_snap: Option<bool>,
    /// DPI override as PROFILE:DPI or PROFILE=DPI, for example --dpi 3:3200.
    #[arg(long, value_parser = parse_dpi)]
    dpi: Vec<(usize, u16)>,
    #[arg(long)]
    active_dpi: Option<u8>,
}

fn main() {
    if std::env::args_os().len() == 1 {
        Cli::command()
            .print_help()
            .expect("stdout must be writable");
        println!();
        return;
    }
    let cli = Cli::parse();
    if cli.gui {
        if let Err(error) = run_gui() {
            eprintln!("ERROR: {error}");
            std::process::exit(1);
        }
        return;
    }
    if let Err(error) = run(cli) {
        eprintln!("ERROR: {error}");
        std::process::exit(1);
    }
}

fn run_gui() -> std::io::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tauri = root.join("ui/node_modules/.bin/tauri");
    let status = Command::new(&tauri)
        .arg("dev")
        .current_dir(root)
        .status()
        .map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!(
                    "could not start Tauri at {} ({error}). Run `npm install --prefix ui` first",
                    tauri.display()
                ),
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("Tauri exited with {status}")))
    }
}

fn run(cli: Cli) -> Result<()> {
    let config_path = cli.config_path.clone().unwrap_or_else(default_config_path);
    let mut config = Config::load(config_path)?;
    apply_overrides(&mut config, &cli)?;
    let mut mouse = DeviceService::open()?;
    if cli.reapply_config {
        mouse.apply_config(&config)?;
    }
    let battery_charge = (!mouse.is_wired())
        .then(|| mouse.read_status().ok())
        .flatten();
    if cli.query_charge {
        println!("{}", battery_charge.unwrap_or(0));
    }
    if cli.polling_rate.is_some() {
        mouse.set_polling_rate(config.polling_rate)?;
    }
    if cli.key_response_time.is_some() || cli.sleep_time.is_some() || cli.deep_sleep_time.is_some()
    {
        mouse.set_times(
            config.sleep_time,
            config.deep_sleep_time,
            config.key_response_time,
        )?;
    }
    if !cli.dpi.is_empty()
        || cli.active_dpi.is_some()
        || cli.ripple_control.is_some()
        || cli.angle_snap.is_some()
    {
        mouse.set_dpis(
            config.dpis,
            config.active_dpi,
            config.ripple_control,
            config.angle_snap,
        )?;
    }
    Ok(())
}

fn default_config_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|path| path.join("attack-shark/config.toml"))
        .unwrap_or_else(|| PathBuf::from("/etc/attack-shark/config.toml"))
}

fn parse_dpi(value: &str) -> std::result::Result<(usize, u16), String> {
    let (profile, dpi) = value
        .split_once(':')
        .or_else(|| value.split_once('='))
        .ok_or_else(|| "expected PROFILE:DPI".to_string())?;
    let profile: usize = profile
        .parse()
        .map_err(|_| "profile must be a number".to_string())?;
    let dpi: u16 = dpi
        .parse()
        .map_err(|_| "DPI must be a number".to_string())?;
    if !(1..=6).contains(&profile) {
        return Err("profile must be between 1 and 6".into());
    }
    Config::validate_dpi(dpi).map_err(|error| error.to_string())?;
    Ok((profile, dpi))
}

fn apply_overrides(config: &mut Config, cli: &Cli) -> Result<()> {
    if let Some(rate) = cli.polling_rate {
        config.polling_rate = PollingRate::from_hz(rate)?;
    }
    if let Some(value) = cli.key_response_time {
        if !(4..=50).contains(&value) || !value.is_multiple_of(2) {
            return Err(DriverError::InvalidConfig(
                "key_response_time must be an even value between 4 and 50".into(),
            ));
        }
        config.key_response_time = value;
    }
    if let Some(value) = cli.sleep_time {
        if !(0.5..=30.0).contains(&value) {
            return Err(DriverError::InvalidConfig(
                "sleep_time must be between 0.5 and 30".into(),
            ));
        }
        config.sleep_time = value;
    }
    if let Some(value) = cli.deep_sleep_time {
        if !(1..=60).contains(&value) {
            return Err(DriverError::InvalidConfig(
                "deep_sleep_time must be between 1 and 60".into(),
            ));
        }
        config.deep_sleep_time = value;
    }
    if let Some(value) = cli.active_dpi {
        if !(1..=6).contains(&value) {
            return Err(DriverError::InvalidConfig(
                "active_dpi must be between 1 and 6".into(),
            ));
        }
        config.active_dpi = value;
    }
    if let Some(value) = cli.ripple_control {
        config.ripple_control = value;
    }
    if let Some(value) = cli.angle_snap {
        config.angle_snap = value;
    }
    for &(profile, dpi) in &cli.dpi {
        config.dpis[profile - 1] = dpi;
    }
    Ok(())
}
