import * as Slider from "@radix-ui/react-slider";
import * as Switch from "@radix-ui/react-switch";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import logoImage from "./assets/attack-shark-logo.webp";
import mouseImage from "./assets/attack-shark.webp";

type PollingRate = "Hz125" | "Hz250" | "Hz500" | "Hz1000";
type ConnectionMode = "wired" | "wireless" | "unknown";
type DeviceStatus = { mode: "wired" | "wireless"; battery_charge: number | null };
type ApplyResult = { skipped: string[] };

type Config = {
  polling_rate: PollingRate;
  dpis: number[];
  active_dpi: number;
  sleep_time: number;
  deep_sleep_time: number;
  key_response_time: number;
  angle_snap: boolean;
  ripple_control: boolean;
};

const defaultConfig: Config = {
  polling_rate: "Hz500",
  dpis: [800, 1600, 3200, 4000, 5000, 12000],
  active_dpi: 3,
  sleep_time: 6,
  deep_sleep_time: 12,
  key_response_time: 4,
  angle_snap: false,
  ripple_control: false,
};

export function App() {
  const [config, setConfig] = useState<Config>(defaultConfig);
  const [battery, setBattery] = useState<number | null>(null);
  const [notice, setNotice] = useState("Ready");
  const [isApplying, setIsApplying] = useState(false);
  const [connectionMode, setConnectionMode] = useState<ConnectionMode>("unknown");
  const applyTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => {
    invoke<Config>("load_config").then(setConfig).catch(() => setNotice("Using local defaults"));
  }, []);

  useEffect(() => {
    if (isApplying) return;
    const refreshDeviceStatus = () => {
      invoke<DeviceStatus>("device_status")
        .then(({ mode, battery_charge }) => {
          setConnectionMode(mode);
          setBattery(battery_charge);
        })
        .catch(() => {
          setConnectionMode("unknown");
          setBattery(null);
        });
    };
    refreshDeviceStatus();
    const interval = window.setInterval(refreshDeviceStatus, 2000);
    return () => window.clearInterval(interval);
  }, [isApplying]);

  const apply = async (nextConfig: Config) => {
    setIsApplying(true);
    setNotice("Applying settings...");
    try {
      const result = await invoke<ApplyResult>("apply_config", { config: nextConfig });
      setNotice(result.skipped.length === 0 ? "Settings applied" : `Applied with skipped items: ${result.skipped.join(", ")}`);
    } catch (error) {
      console.error("Failed to apply mouse configuration:", error);
      setNotice(String(error));
    } finally {
      setIsApplying(false);
    }
  };

  const update = <K extends keyof Config>(key: K, value: Config[K]) => {
    if (isApplying) return;
    const nextConfig = { ...config, [key]: value } as Config;
    setConfig(nextConfig);
    setNotice("Applying changes...");
    clearTimeout(applyTimer.current);
    applyTimer.current = setTimeout(() => void apply(nextConfig), 350);
  };

  return (
    <main className="app-shell">
      {isApplying && <div className="loading-overlay" role="status" aria-live="assertive"><div className="loader" /><span>Applying settings to mouse...</span></div>}
      <header className="titlebar" onMouseDown={(event) => { if (event.button === 0) void getCurrentWindow().startDragging(); }}>
        <div className="brand"><img src={logoImage} alt="Attack Shark" /></div>
        <div className="connection" aria-hidden="true" />
        <div className="window-controls">
          <ConnectionIndicator mode={connectionMode} />
          <button aria-label="Minimize" onMouseDown={(event) => event.stopPropagation()} onClick={() => void getCurrentWindow().minimize()}>−</button>
          <button aria-label="Close" className="close" onMouseDown={(event) => event.stopPropagation()} onClick={() => void getCurrentWindow().close()}>×</button>
        </div>
      </header>

      <section className="control-deck">
        <aside className="left-column">
          <Panel title="Power">
            <div className="battery-wrap"><div className="battery"><div style={{ width: `${battery ?? 0}%` }} /></div><strong>{battery === null ? "—" : `${battery}%`}</strong></div>
          </Panel>
          <Panel title="Mouse Attribute"><Settings config={config} update={update} /></Panel>
        </aside>

        <section className="mouse-stage">
          <MouseDiagram />
          <p className="status-notice" role="status">{notice}</p>
        </section>

        <aside className="right-column">
          <Panel title="DPI Settings"><DpiEditor config={config} update={update} /></Panel>
          <Panel title="Polling Rate Settings"><PollingControl value={config.polling_rate} update={(value) => update("polling_rate", value)} /></Panel>
        </aside>
      </section>
    </main>
  );
}

function ConnectionIndicator({ mode }: { mode: ConnectionMode }) {
  const label = mode === "wired" ? "Connected by cable" : mode === "wireless" ? "Connected by dongle" : "Mouse connection unavailable";
  const icon = mode === "wired" ? "usb" : mode === "wireless" ? "wifi" : "help";
  return <span className={`connection-indicator ${mode}`} title={label} aria-label={label}><span className="material-symbols-rounded">{icon}</span></span>;
}

function Panel({ title, children }: { title: string; children: React.ReactNode }) {
  return <section className="panel"><h2>{title}<span>−</span></h2><div className="panel-content">{children}</div></section>;
}

function DpiEditor({ config, update }: { config: Config; update: <K extends keyof Config>(key: K, value: Config[K]) => void }) {
  return <div className="dpi-editor">{config.dpis.map((dpi, index) => <label className="dpi-row" key={index}><input type="checkbox" checked={config.active_dpi === index + 1} onChange={(event) => { if (event.target.checked) update("active_dpi", index + 1); }} /><span>DPI {index + 1}</span><input type="number" min="100" max="18000" step="100" value={dpi} onChange={(event) => { const dpis = [...config.dpis]; dpis[index] = Number(event.target.value); update("dpis", dpis); }} /></label>)}</div>;
}

function PollingControl({ value, update }: { value: PollingRate; update: (value: PollingRate) => void }) {
  const options: { value: PollingRate; label: string }[] = [{ value: "Hz125", label: "125Hz" }, { value: "Hz250", label: "250Hz" }, { value: "Hz500", label: "500Hz" }, { value: "Hz1000", label: "1000Hz" }];
  return <div className="polling-control"><div className="polling-wheel">{options.map((option) => <button key={option.value} className={value === option.value ? "active" : ""} onClick={() => update(option.value)}>{option.label}</button>)}<div className="wheel-center">E-sports</div></div></div>;
}

function Settings({ config, update }: { config: Config; update: <K extends keyof Config>(key: K, value: Config[K]) => void }) {
  return <div className="settings"><Range label="Sleep Time" value={config.sleep_time} min={0.5} max={30} step={0.5} suffix="" onChange={(value) => update("sleep_time", value)} /><Range label="Key Response Time" value={config.key_response_time} min={4} max={50} step={2} suffix=" ms" onChange={(value) => update("key_response_time", value)} /><Range label="Deep Sleep Time" value={config.deep_sleep_time} min={1} max={60} step={1} suffix="" onChange={(value) => update("deep_sleep_time", value)} /><Toggle label="Ripple Control" checked={config.ripple_control} onChange={(value) => update("ripple_control", value)} /><Toggle label="Angle Snap" checked={config.angle_snap} onChange={(value) => update("angle_snap", value)} /></div>;
}

function Range({ label, value, min, max, step, suffix, onChange }: { label: string; value: number; min: number; max: number; step: number; suffix: string; onChange: (value: number) => void }) {
  return <div className="range-control"><div><span>{label}</span><b>{value}{suffix}</b></div><Slider.Root className="slider" value={[value]} min={min} max={max} step={step} onValueChange={([next]) => onChange(next)}><Slider.Track className="slider-track"><Slider.Range className="slider-range" /></Slider.Track><Slider.Thumb className="slider-thumb" /></Slider.Root></div>;
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <label className="toggle"><span>{label}</span><Switch.Root className="switch" checked={checked} onCheckedChange={onChange}><Switch.Thumb className="switch-thumb" /></Switch.Root></label>;
}

function MouseDiagram() {
  return <img className="mouse-image" src={mouseImage} alt="Attack Shark wireless mouse" />;
}
