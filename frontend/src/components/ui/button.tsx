import type { ComponentProps } from "react";
import { type VariantProps, cva } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 rounded-sm font-semibold whitespace-nowrap outline-hidden transition-[background,filter,color] duration-100 ease-snap disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-surface-3 text-fg-1 shadow-card-flat hover:bg-surface-hover",
        primary: "bg-hearth-500 text-on-hearth hover:brightness-107",
        play: "bg-grass-500 text-on-grass shadow-bevel-btn hover:brightness-108 active:translate-y-px",
        ghost: "bg-transparent text-fg-2 hover:bg-surface-hover hover:text-fg-1",
        danger:
          "bg-transparent text-tnt-400 shadow-[inset_0_0_0_1px_var(--color-tnt-700)] hover:bg-tnt-500/16",
      },
      size: {
        /* Hero-type box for primary launch actions (DS --control-h-lg). */
        lg: "h-12 gap-2.5 px-6 font-hero text-lg tracking-wide font-crisp",
        md: "h-9 px-4 text-sm",
        sm: "h-7 px-3 text-xs",
      },
    },
    defaultVariants: { variant: "default", size: "md" },
  },
);

function Button({
  className,
  variant,
  size,
  ...props
}: ComponentProps<"button"> & VariantProps<typeof buttonVariants>) {
  return (
    <button
      data-slot="button"
      className={cn(buttonVariants({ variant, size }), className)}
      {...props}
    />
  );
}

interface IconButtonProps extends ComponentProps<"button"> {
  quiet?: boolean;
  active?: boolean;
}

/** Square 36px icon-only control: solid by default, transparent when `quiet`. */
function IconButton({ quiet = false, active = false, className, ...props }: IconButtonProps) {
  return (
    <button
      data-slot="icon-button"
      className={cn(
        "inline-flex size-9 items-center justify-center rounded-sm outline-hidden transition-colors duration-100 ease-snap",
        active
          ? "bg-surface-active text-hearth-400"
          : quiet
            ? "bg-transparent text-fg-2 hover:bg-surface-hover hover:text-fg-1"
            : "bg-surface-3 text-fg-2 shadow-card-flat hover:bg-surface-hover hover:text-fg-1",
        className,
      )}
      {...props}
    />
  );
}

export { Button, IconButton, buttonVariants };
