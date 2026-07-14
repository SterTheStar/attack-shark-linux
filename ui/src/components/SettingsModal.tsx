import type { Translation } from "../i18n";
import type { DeviceModel, Language, MouseSelection } from "../types";
import { ThemeDropdown } from "./ThemeDropdown";

export function SettingsModal({
  onboarding,
  selection,
  detectedModel,
  language,
  t,
  onSelection,
  onLanguage,
  onClose,
}: {
  onboarding: boolean;
  selection: MouseSelection;
  detectedModel: DeviceModel;
  language: Language;
  t: Translation;
  onSelection: (selection: MouseSelection) => void;
  onLanguage: (language: Language) => void;
  onClose: () => void;
}) {
  const detected = detectedModel === "r1" ? t.modelR1 : detectedModel === "x11" ? t.modelX11 : t.compatibleAdapter;
  return <div className="settings-backdrop" role="dialog" aria-modal="true" aria-label={t.settings}>
    <section className="settings-modal">
      <header className="settings-modal-header"><h2>{onboarding ? t.onboardingTitle : t.settings}</h2>{!onboarding && <button aria-label={t.close} onClick={onClose}>×</button>}</header>
      <p className="settings-description">{onboarding ? t.onboardingDescription : t.appDescription}</p>
      <div className="settings-options">
        <label className="settings-row"><span>{t.mouseSelection}</span><ThemeDropdown value={selection} onChange={onSelection} options={[{ value: "auto", label: `${t.automatic} (${detected})` }, { value: "r1", label: t.modelR1 }, { value: "x11", label: t.modelX11 }]} /></label>
        <label className="settings-row"><span>{t.language}</span><ThemeDropdown value={language} onChange={onLanguage} options={[{ value: "en", label: t.english }, { value: "pt", label: t.portuguese }]} /></label>
      </div>
      <section className="settings-about"><h3>{t.about}</h3><p>{t.appDescription}</p><dl><div><dt>{t.version}</dt><dd>1.0.1</dd></div><div><dt>{t.developers}</dt><dd>SterTheStar</dd></div></dl><small>{t.protocolCredits}</small></section>
      {onboarding && <button className="settings-primary" onClick={onClose}>{t.getStarted}</button>}
    </section>
  </div>;
}
