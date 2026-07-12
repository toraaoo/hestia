import type { ButtonHTMLAttributes } from "react";

export type ButtonVariant = "default" | "primary" | "play" | "ghost" | "danger";
export type ButtonSize = "md" | "sm";

const VARIANTS: Record<ButtonVariant, string> = {
  default: "bg-surface-3 text-text-1 shadow-card-flat hover:bg-surface-hover",
  primary: "bg-hearth-500 text-on-hearth hover:brightness-107",
  play: "bg-grass-500 text-on-grass hover:brightness-107",
  ghost: "bg-transparent text-text-2 hover:bg-surface-hover hover:text-text-1",
  danger:
    "bg-transparent text-tnt-400 shadow-[inset_0_0_0_1px_var(--color-tnt-700)] hover:bg-tnt-500/16",
};

/* control heights from the DS spacing tokens: md 36px, sm 28px */
const SIZES: Record<ButtonSize, string> = {
  md: "h-9 px-4 text-sm",
  sm: "h-7 px-3 text-xs",
};

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
}

export function Button({ variant = "default", size = "md", className = "", ...rest }: ButtonProps) {
  return (
    <button
      className={`inline-flex items-center justify-center gap-2 rounded-sm font-semibold whitespace-nowrap transition-[background,filter,color] duration-100 ease-snap disabled:pointer-events-none disabled:opacity-50 ${VARIANTS[variant]} ${SIZES[size]} ${className}`}
      {...rest}
    />
  );
}

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  quiet?: boolean;
  active?: boolean;
}

export function IconButton({
  quiet = false,
  active = false,
  className = "",
  ...rest
}: IconButtonProps) {
  const surface = active
    ? "bg-surface-active text-hearth-400"
    : quiet
      ? "bg-transparent text-text-2 hover:bg-surface-hover hover:text-text-1"
      : "bg-surface-3 text-text-2 shadow-card-flat hover:bg-surface-hover hover:text-text-1";
  return (
    <button
      className={`inline-flex size-9 items-center justify-center rounded-sm transition-colors duration-100 ease-snap ${surface} ${className}`}
      {...rest}
    />
  );
}
