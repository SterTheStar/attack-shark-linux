export function ThemeNumberInput({
  value,
  min,
  max,
  step,
  disabled,
  onValueChange,
  onBlur,
}: {
  value: number;
  min: number;
  max: number;
  step: number;
  disabled: boolean;
  onValueChange: (value: number) => void;
  onBlur: (value: number) => void;
}) {
  const adjust = (amount: number) => onValueChange(Math.min(max, Math.max(min, value + amount)));
  return <div className="theme-number-input">
    <input type="number" disabled={disabled} min={min} max={max} step={step} value={value} onChange={(event) => onValueChange(Number(event.target.value))} onBlur={(event) => onBlur(Number(event.currentTarget.value))} />
    <span className="theme-number-stepper"><button type="button" aria-label="Increase value" disabled={disabled} onMouseDown={(event) => event.preventDefault()} onClick={() => adjust(step)} /><button type="button" aria-label="Decrease value" disabled={disabled} onMouseDown={(event) => event.preventDefault()} onClick={() => adjust(-step)} /></span>
  </div>;
}
