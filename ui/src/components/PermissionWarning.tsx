import { useState } from "react";
import type { Translation } from "../i18n";
import type { UdevRuleStatus } from "../types";

export function PermissionWarning({ rule, t }: { rule: UdevRuleStatus; t: Translation }) {
  const [copied, setCopied] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const copy = async () => {
    await navigator.clipboard.writeText(rule.command);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1800);
  };

  if (dismissed) return null;

  return <section className="permission-warning" role="alert">
    <header><strong>{t.permissionTitle}</strong><button aria-label={t.close} onClick={() => setDismissed(true)}>×</button></header>
    <div className="permission-warning-content">
      <p>{t.permissionDescription}</p>
      <div className="permission-command"><code>{rule.command}</code><button onClick={() => void copy()}>{copied ? t.copied : t.copyCommand}</button></div>
    </div>
  </section>;
}
