import type { ButtonHTMLAttributes } from "react";
import { cn } from "../../lib/cn";
import { PlayIcon } from "../icons";

/** The big grass launch action (48px hero-type button with the pixel bevel). */
export function PlayButton({
  className = "",
  children,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      className={cn(
        "flex h-12 items-center justify-center gap-2.5 rounded-lg bg-grass-500 px-6.5 font-hero text-lg tracking-wide text-on-grass font-crisp shadow-bevel-btn transition-[filter,transform] duration-100 ease-snap hover:brightness-108 active:translate-y-px",
        className,
      )}
      {...rest}
    >
      <PlayIcon size={16} weight="fill" />
      {children ?? "PLAY"}
    </button>
  );
}
