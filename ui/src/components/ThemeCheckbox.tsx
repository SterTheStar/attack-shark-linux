export function ThemeCheckbox({
  checked,
  disabled,
  onCheckedChange,
}: {
  checked: boolean;
  disabled: boolean;
  onCheckedChange: (checked: boolean) => void;
}) {
  return <input className="theme-checkbox" type="checkbox" disabled={disabled} checked={checked} onChange={(event) => onCheckedChange(event.target.checked)} />;
}
