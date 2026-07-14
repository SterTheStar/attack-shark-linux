import type { ConnectionMode } from "../types";
import type { Translation } from "../i18n";

export function ConnectionIndicator({ mode, t }: { mode: ConnectionMode; t: Translation }) {
  const label = mode === "wired" ? t.connectionCable : mode === "wireless" ? t.connectionDongle : t.connectionUnavailable;
  const icon = mode === "wired" ? "usb" : mode === "wireless" ? "wifi" : "help";
  return <span className={`connection-indicator ${mode}`} title={label} aria-label={label}><span className="material-symbols-rounded">{icon}</span></span>;
}
