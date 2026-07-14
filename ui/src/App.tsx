import * as Slider from "@radix-ui/react-slider";
import * as Switch from "@radix-ui/react-switch";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useRef, useState } from "react";
import logoImage from "./assets/attack-shark-logo.webp";
import mouseImage from "./assets/attack-shark.webp";
import { BetaDisclaimer } from "./components/BetaDisclaimer";
import { ConnectionIndicator } from "./components/ConnectionIndicator";
import { PermissionWarning } from "./components/PermissionWarning";
import { SettingsModal } from "./components/SettingsModal";
import { ThemeCheckbox } from "./components/ThemeCheckbox";
import { ThemeDropdown } from "./components/ThemeDropdown";
import { ThemeNumberInput } from "./components/ThemeNumberInput";
import { translations, type Translation } from "./i18n";
import type { ApplyResult, Config, ConnectionMode, DeviceModel, DeviceStatus, Language, LedMode, MouseSelection, PollingRate, UdevRuleStatus } from "./types";

const LANGUAGE_KEY = "attack-shark.language";
const MODEL_KEY = "attack-shark.mouse-selection";
const BETA_NOTICE_KEY = "attack-shark.beta-notice-accepted";
const MINIMUM_APPLY_DURATION = 2000;

const defaultConfig: Config = {
  polling_rate: "Hz500",
  dpis: [800, 1600, 3200, 4000, 5000, 12000],
  active_dpi: 3,
  sleep_time: 6,
  deep_sleep_time: 12,
  key_response_time: 4,
  angle_snap: false,
  ripple_control: false,
  led_mode: "Disabled",
  led_color: [0, 255, 0],
  dpi_colors: [[255, 0, 0], [0, 255, 0], [0, 255, 255], [255, 0, 0], [0, 255, 255], [64, 0, 255]],
  led_speed: 3,
};

export function App() {
  const [config, setConfig] = useState<Config>(defaultConfig);
  const [battery, setBattery] = useState<number | null>(null);
  const [notice, setNotice] = useState("Ready");
  const [isApplying, setIsApplying] = useState(false);
  const [connectionMode, setConnectionMode] = useState<ConnectionMode>("unknown");
  const [deviceModel, setDeviceModel] = useState<DeviceModel>("unknown");
  const [udevRule, setUdevRule] = useState<UdevRuleStatus | null>(null);
  const [language, setLanguage] = useState<Language>(() => (localStorage.getItem(LANGUAGE_KEY) as Language) || "en");
  const [selection, setSelection] = useState<MouseSelection>(() => (localStorage.getItem(MODEL_KEY) as MouseSelection) || "auto");
  const [showSettings, setShowSettings] = useState(false);
  const [onboarding, setOnboarding] = useState(() => !localStorage.getItem(MODEL_KEY));
  const [betaNoticeAccepted, setBetaNoticeAccepted] = useState(() => localStorage.getItem(BETA_NOTICE_KEY) === "true");
  const applyTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const t = translations[language];
  const canConfigure = connectionMode !== "unknown" && udevRule?.installed === true;

  useEffect(() => {
    invoke<Config>("load_config").then((savedConfig) => { setConfig(savedConfig); setNotice(t.savedSettingsLoaded); }).catch(() => setNotice(t.usingLocalDefaults));
  }, [t.savedSettingsLoaded, t.usingLocalDefaults]);

  useEffect(() => {
    if (isApplying) return;
    const refreshDeviceStatus = () => {
      invoke<DeviceStatus>("device_status", { modelOverride: selection === "auto" ? null : selection })
        .then(({ model, mode, battery_charge, active_dpi, udev_rule }) => {
          setDeviceModel(model);
          setConnectionMode(mode);
          setBattery(battery_charge);
          setUdevRule(udev_rule);
          if (model === "x11" && active_dpi) setConfig((current) => current.active_dpi === active_dpi ? current : { ...current, active_dpi });
        })
        .catch(() => {
          setDeviceModel("unknown");
          setConnectionMode("unknown");
          setBattery(null);
          setUdevRule(null);
        });
    };
    refreshDeviceStatus();
    const interval = window.setInterval(refreshDeviceStatus, 2000);
    return () => window.clearInterval(interval);
  }, [isApplying, selection]);

  const apply = async (nextConfig: Config) => {
    const startedAt = Date.now();
    setIsApplying(true);
    setNotice("Applying settings...");
    try {
      const result = await invoke<ApplyResult>("apply_config", { config: nextConfig, modelOverride: selection === "auto" ? null : selection });
      setNotice(result.skipped.length === 0 ? t.settingsApplied : `${t.skipped} ${result.skipped.join(", ")}`);
    } catch (error) {
      console.error("Failed to apply mouse configuration:", error);
      setNotice(String(error));
    } finally {
      const remaining = MINIMUM_APPLY_DURATION - (Date.now() - startedAt);
      if (remaining > 0) await new Promise((resolve) => window.setTimeout(resolve, remaining));
      setIsApplying(false);
    }
  };

  const scheduleApply = (nextConfig: Config, delay: number) => {
    clearTimeout(applyTimer.current);
    setNotice(t.applyingChanges);
    applyTimer.current = setTimeout(() => void apply(nextConfig), delay);
  };

  const setConfigValue = <K extends keyof Config>(key: K, value: Config[K]) => {
    if (isApplying || !canConfigure) return;
    const nextConfig = { ...config, [key]: value } as Config;
    setConfig(nextConfig);
    return nextConfig;
  };

  const update = <K extends keyof Config>(key: K, value: Config[K]) => {
    const nextConfig = setConfigValue(key, value);
    if (nextConfig) scheduleApply(nextConfig, 350);
  };

  const previewUpdate = <K extends keyof Config>(key: K, value: Config[K]) => {
    const nextConfig = setConfigValue(key, value);
    if (nextConfig) clearTimeout(applyTimer.current);
  };

  const deferUpdate = <K extends keyof Config>(key: K, value: Config[K]) => {
    const nextConfig = setConfigValue(key, value);
    if (nextConfig) scheduleApply(nextConfig, 3000);
  };

  const commitUpdate = <K extends keyof Config>(key: K, value: Config[K]) => {
    const nextConfig = setConfigValue(key, value);
    if (nextConfig) scheduleApply(nextConfig, 0);
  };

  return (
    <main className="app-shell">
      {isApplying && <div className="loading-overlay" role="status" aria-live="assertive"><div className="loader" /><span>{t.applying}</span></div>}
      {!betaNoticeAccepted && <BetaDisclaimer t={t} onAccepted={() => { localStorage.setItem(BETA_NOTICE_KEY, "true"); setBetaNoticeAccepted(true); }} />}
      {betaNoticeAccepted && (onboarding || showSettings) && <SettingsModal onboarding={onboarding} selection={selection} detectedModel={deviceModel} language={language} t={t} onSelection={(value) => { setSelection(value); localStorage.setItem(MODEL_KEY, value); }} onLanguage={(value) => { setLanguage(value); localStorage.setItem(LANGUAGE_KEY, value); }} onClose={() => { if (onboarding) localStorage.setItem(MODEL_KEY, selection); setOnboarding(false); setShowSettings(false); }} />}
      <header className="titlebar" onMouseDown={(event) => { if (event.button === 0) void getCurrentWindow().startDragging(); }}>
        <div className="brand"><img src={logoImage} alt="Attack Shark" /></div>
        <div className="connection" aria-hidden="true" />
        <div className="window-controls">
          <ConnectionIndicator mode={connectionMode} t={t} />
          <button aria-label={t.settings} onMouseDown={(event) => event.stopPropagation()} onClick={() => setShowSettings(true)}><span className="material-symbols-rounded">settings</span></button>
          <button aria-label="Minimize" onMouseDown={(event) => event.stopPropagation()} onClick={() => void getCurrentWindow().minimize()}><span className="material-symbols-rounded">remove</span></button>
          <button aria-label="Close" className="close" onMouseDown={(event) => event.stopPropagation()} onClick={() => void getCurrentWindow().close()}><span className="material-symbols-rounded">close</span></button>
        </div>
      </header>

      {udevRule && !udevRule.installed && deviceModel !== "unknown" && <PermissionWarning rule={udevRule} t={t} />}

      <section className="control-deck">
        <aside className="left-column">
          <Panel title={t.power}>
            <div className={`battery-wrap${connectionMode === "wired" ? " is-charging" : ""}`}><div className="battery"><div style={{ width: `${battery ?? 0}%` }} /></div><strong>{connectionMode === "wired" ? t.charging : battery === null ? "—" : `${battery}%`}</strong></div>
          </Panel>
          <Panel title={t.mouseAttributes}><Settings config={config} update={update} previewUpdate={previewUpdate} deferUpdate={deferUpdate} commitUpdate={commitUpdate} t={t} disabled={!canConfigure} /></Panel>
          {deviceModel === "x11" && <Panel title={t.ledEffect}><LedControl config={config} update={update} commitUpdate={commitUpdate} disabled={!canConfigure} t={t} /></Panel>}
        </aside>

        <section className="mouse-stage">
          <MouseDiagram />
          <p className="device-model">{deviceModelLabel(deviceModel, connectionMode, t)}</p>
          <p className="status-notice" role="status">{notice}</p>
        </section>

        <aside className="right-column">
          <Panel title={t.dpiSettings}><DpiEditor config={config} model={deviceModel} update={update} deferUpdate={deferUpdate} commitUpdate={commitUpdate} disabled={!canConfigure} /></Panel>
          <Panel title={t.pollingRate}><PollingControl value={config.polling_rate} update={(value) => update("polling_rate", value)} disabled={!canConfigure} /></Panel>
        </aside>
      </section>
      <footer className="app-footer">Made with &lt;3 by <button onClick={() => void openUrl("https://x.com/onlysterbr").catch(console.error)}>Esther</button></footer>
    </main>
  );
}

function deviceModelLabel(model: DeviceModel, mode: ConnectionMode, t: Translation) {
  if (mode === "unknown") return t.noMouse;
  if (model === "r1") return t.modelR1;
  if (model === "x11") return t.modelX11;
  if (model === "adapter") return t.compatibleAdapter;
  return t.noMouse;
}

function Panel({ title, children }: { title: string; children: React.ReactNode }) {
  const [expanded, setExpanded] = useState(true);
  return <section className="panel"><h2><span>{title}</span><button className="panel-toggle" aria-label={expanded ? `Collapse ${title}` : `Expand ${title}`} aria-expanded={expanded} onClick={() => setExpanded(!expanded)}>{expanded ? "−" : "+"}</button></h2>{expanded && <div className="panel-content">{children}</div>}</section>;
}

function DpiEditor({ config, model, update, deferUpdate, commitUpdate, disabled }: { config: Config; model: DeviceModel; update: <K extends keyof Config>(key: K, value: Config[K]) => void; deferUpdate: <K extends keyof Config>(key: K, value: Config[K]) => void; commitUpdate: <K extends keyof Config>(key: K, value: Config[K]) => void; disabled: boolean }) {
  const isX11 = model === "x11";
  return <div className={`dpi-editor${disabled ? " is-disabled" : ""}`}>{config.dpis.map((dpi, index) => <label className={`dpi-row${isX11 ? " has-color" : ""}`} key={index}><ThemeCheckbox disabled={disabled} checked={config.active_dpi === index + 1} onCheckedChange={(checked) => { if (checked) update("active_dpi", index + 1); }} />{isX11 && <input className="dpi-color" aria-label={`DPI ${index + 1} color`} disabled={disabled} type="color" value={rgbToHex(config.dpi_colors[index])} onChange={(event) => { const colors = config.dpi_colors.map((color) => [...color]) as Config["dpi_colors"]; colors[index] = hexToRgb(event.target.value); update("dpi_colors", colors); }} />}<span>DPI {index + 1}</span><ThemeNumberInput disabled={disabled} min={isX11 ? 50 : 100} max={isX11 ? 22000 : 18000} step={isX11 && dpi < 10000 ? 50 : 100} value={dpi} onValueChange={(value) => { const dpis = [...config.dpis]; dpis[index] = value; deferUpdate("dpis", dpis); }} onBlur={(value) => { const dpis = [...config.dpis]; dpis[index] = value; commitUpdate("dpis", dpis); }} /></label>)}</div>;
}

function PollingControl({ value, update, disabled }: { value: PollingRate; update: (value: PollingRate) => void; disabled: boolean }) {
  const options: { value: PollingRate; label: string }[] = [{ value: "Hz125", label: "125Hz" }, { value: "Hz250", label: "250Hz" }, { value: "Hz500", label: "500Hz" }, { value: "Hz1000", label: "1000Hz" }];
  return <div className={`polling-control${disabled ? " is-disabled" : ""}`}><div className="polling-wheel">{options.map((option) => <button key={option.value} disabled={disabled} className={value === option.value ? "active" : ""} onClick={() => update(option.value)}>{option.label}</button>)}<div className="wheel-center">E-sports</div></div></div>;
}

function LedControl({ config, update, commitUpdate, disabled, t }: { config: Config; update: <K extends keyof Config>(key: K, value: Config[K]) => void; commitUpdate: <K extends keyof Config>(key: K, value: Config[K]) => void; disabled: boolean; t: Translation }) {
  const usesColor = config.led_mode === "Static" || config.led_mode === "Breathing";
  const usesSpeed = ["Breathing", "Neon", "ColorBreathing", "BreathingDpi"].includes(config.led_mode);
  return <div className={`led-control${disabled ? " is-disabled" : ""}`}><ThemeDropdown disabled={disabled} value={config.led_mode} onChange={(value) => update("led_mode", value)} options={[{ value: "Disabled", label: t.ledDisabled }, { value: "Static", label: t.ledStatic }, { value: "Breathing", label: t.ledBreathing }, { value: "Neon", label: t.ledNeon }, { value: "ColorBreathing", label: t.ledColorBreathing }, { value: "StaticDpi", label: t.ledStaticDpi }, { value: "BreathingDpi", label: t.ledBreathingDpi }]} />{usesColor && <label className="led-color-row"><span>{t.ledColor}</span><input disabled={disabled} type="color" value={rgbToHex(config.led_color)} onChange={(event) => update("led_color", hexToRgb(event.target.value))} /></label>}{usesSpeed && <div className="led-speed"><span>{t.ledSpeed}</span><ThemeNumberInput disabled={disabled} min={1} max={5} step={1} value={config.led_speed} onValueChange={(value) => update("led_speed", value)} onBlur={(value) => commitUpdate("led_speed", value)} /></div>}{(config.led_mode === "StaticDpi" || config.led_mode === "BreathingDpi") && <small>{t.ledUsesDpiColor}</small>}</div>;
}

function rgbToHex(color: number[]) { return `#${color.map((channel) => channel.toString(16).padStart(2, "0")).join("")}`; }
function hexToRgb(value: string): [number, number, number] { return [Number.parseInt(value.slice(1, 3), 16), Number.parseInt(value.slice(3, 5), 16), Number.parseInt(value.slice(5, 7), 16)]; }

function Settings({ config, update, previewUpdate, deferUpdate, commitUpdate, t, disabled }: { config: Config; update: <K extends keyof Config>(key: K, value: Config[K]) => void; previewUpdate: <K extends keyof Config>(key: K, value: Config[K]) => void; deferUpdate: <K extends keyof Config>(key: K, value: Config[K]) => void; commitUpdate: <K extends keyof Config>(key: K, value: Config[K]) => void; t: Translation; disabled: boolean }) {
  return <div className={`settings${disabled ? " is-disabled" : ""}`}><Range label={t.sleepTime} value={config.sleep_time} min={0.5} max={30} step={0.5} suffix=" min" disabled={disabled} onChange={(value) => previewUpdate("sleep_time", value)} onCommit={(value) => commitUpdate("sleep_time", value)} /><Range label={t.keyResponse} value={config.key_response_time} min={4} max={50} step={2} suffix=" ms" disabled={disabled} onChange={(value) => previewUpdate("key_response_time", value)} onCommit={(value) => commitUpdate("key_response_time", value)} /><Range label={t.deepSleepTime} value={config.deep_sleep_time} min={1} max={60} step={1} suffix=" min" disabled={disabled} onChange={(value) => previewUpdate("deep_sleep_time", value)} onCommit={(value) => commitUpdate("deep_sleep_time", value)} /><Toggle label={t.rippleControl} checked={config.ripple_control} disabled={disabled} onChange={(value) => update("ripple_control", value)} /><Toggle label={t.angleSnap} checked={config.angle_snap} disabled={disabled} onChange={(value) => update("angle_snap", value)} /></div>;
}

function Range({ label, value, min, max, step, suffix, disabled, onChange, onCommit }: { label: string; value: number; min: number; max: number; step: number; suffix: string; disabled: boolean; onChange: (value: number) => void; onCommit: (value: number) => void }) {
  return <div className="range-control"><div><span>{label}</span><b>{value}{suffix}</b></div><Slider.Root className="slider" disabled={disabled} value={[value]} min={min} max={max} step={step} onValueChange={([next]) => onChange(next)} onValueCommit={([next]) => onCommit(next)}><Slider.Track className="slider-track"><Slider.Range className="slider-range" /></Slider.Track><Slider.Thumb className="slider-thumb" /></Slider.Root></div>;
}

function Toggle({ label, checked, disabled, onChange }: { label: string; checked: boolean; disabled: boolean; onChange: (value: boolean) => void }) {
  return <label className="toggle"><span>{label}</span><Switch.Root className="switch" disabled={disabled} checked={checked} onCheckedChange={onChange}><Switch.Thumb className="switch-thumb" /></Switch.Root></label>;
}

function MouseDiagram() {
  return <img className="mouse-image" src={mouseImage} alt="Attack Shark wireless mouse" />;
}
