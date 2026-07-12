interface ToggleProps {
  on: boolean;
  onChange: (on: boolean) => void;
  label?: string;
}

export function Toggle({ on, onChange, label }: ToggleProps) {
  return (
    <button
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={() => onChange(!on)}
      className={`relative h-5.5 w-10 shrink-0 rounded-full shadow-bevel-inset transition-colors duration-100 ${
        on ? "bg-hearth-500/42" : "bg-surface-inset"
      }`}
    >
      <span
        className={`absolute top-0.5 size-4.5 rounded-full transition-[left,background] duration-100 ease-snap ${
          on ? "left-5 bg-hearth-500" : "left-0.5 bg-ink-300"
        }`}
      />
    </button>
  );
}
