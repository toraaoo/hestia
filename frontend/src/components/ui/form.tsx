import type { InputHTMLAttributes, ReactNode } from "react";
import { CaretDownIcon, CheckIcon } from "@/components/icons";

export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-2">
      <span className="text-sm font-semibold text-text-1">{label}</span>
      {children}
      {hint && <span className="text-xs text-text-3">{hint}</span>}
    </div>
  );
}

export function TextInput({ className = "", ...rest }: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={`h-9 rounded-sm bg-surface-inset px-3 text-sm text-text-1 shadow-bevel-inset outline-none ${className}`}
      {...rest}
    />
  );
}

/** Static select facade — becomes a real dropdown once wired to the daemon. */
export function Select({ value }: { value: string }) {
  return (
    <button className="flex h-9 items-center justify-between rounded-sm bg-surface-inset px-3 text-sm text-text-1 shadow-bevel-inset">
      {value}
      <CaretDownIcon size={15} className="text-text-3" />
    </button>
  );
}

export function CheckLabel({
  children,
  className = "",
  ...rest
}: InputHTMLAttributes<HTMLInputElement> & { children: ReactNode }) {
  return (
    <label className="group flex w-fit cursor-pointer items-center gap-2.5 text-sm text-text-2">
      <span className="relative grid size-4 shrink-0 place-items-center">
        <input
          type="checkbox"
          className={`peer size-4 cursor-pointer appearance-none rounded-xs bg-surface-inset shadow-bevel-inset transition-colors checked:bg-hearth-500 disabled:cursor-not-allowed disabled:opacity-50 ${className}`}
          {...rest}
        />
        <CheckIcon
          weight="bold"
          size={12}
          className="pointer-events-none absolute text-on-hearth opacity-0 transition-opacity peer-checked:opacity-100"
        />
      </span>
      {children}
    </label>
  );
}

export function RangeInput(props: InputHTMLAttributes<HTMLInputElement>) {
  return <input type="range" className="w-full accent-hearth-500" {...props} />;
}
