import type { Translation } from "../i18n";
import { useState } from "react";
import { ThemeCheckbox } from "./ThemeCheckbox";

export function BetaDisclaimer({ t, onAccepted }: { t: Translation; onAccepted: () => void }) {
  const [accepted, setAccepted] = useState(false);

  return <div className="settings-backdrop" role="dialog" aria-modal="true" aria-labelledby="beta-notice-title">
    <section className="settings-modal beta-disclaimer">
      <header className="settings-modal-header"><h2 id="beta-notice-title">{t.betaTitle}</h2></header>
      <div className="beta-disclaimer-content">
        <p>{t.betaDescription}</p>
        <p>{t.betaLiability}</p>
        <label className="beta-confirmation"><ThemeCheckbox checked={accepted} disabled={false} onCheckedChange={setAccepted} /><span>{t.betaConfirm}</span></label>
        <button className="settings-primary beta-continue" disabled={!accepted} onClick={onAccepted}>{t.betaContinue}</button>
      </div>
    </section>
  </div>;
}
