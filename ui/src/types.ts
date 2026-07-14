export type PollingRate = "Hz125" | "Hz250" | "Hz500" | "Hz1000";
export type ConnectionMode = "wired" | "wireless" | "unknown";
export type DeviceModel = "r1" | "x11" | "adapter" | "unknown";
export type MouseSelection = "auto" | "r1" | "x11";
export type Language = "en" | "pt";

export type Config = {
  polling_rate: PollingRate;
  dpis: number[];
  active_dpi: number;
  sleep_time: number;
  deep_sleep_time: number;
  key_response_time: number;
  angle_snap: boolean;
  ripple_control: boolean;
};

export type UdevRuleStatus = {
  installed: boolean;
  rule_name: string;
  command: string;
};

export type DeviceStatus = {
  model: Exclude<DeviceModel, "unknown">;
  mode: ConnectionMode;
  battery_charge: number | null;
  udev_rule: UdevRuleStatus;
};

export type ApplyResult = { skipped: string[] };
