import type { ComponentProps } from "react";
import { type VariantProps, cva } from "class-variance-authority";
import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex h-5 items-center gap-1 rounded-xs px-1.5 font-pixel text-xs leading-none tracking-wide whitespace-nowrap uppercase font-crisp shadow-outline-dark",
  {
    variants: {
      tone: {
        neutral: "bg-surface-3 text-fg-2",
        success: "bg-grass-500/22 text-grass-400",
        warning: "bg-gold-500/22 text-gold-400",
        danger: "bg-tnt-500/22 text-tnt-400",
        info: "bg-diamond-500/22 text-diamond-400",
        hearth: "bg-hearth-500/22 text-hearth-300",
        fabric: "bg-loader-fabric/20 text-loader-fabric",
        forge: "bg-loader-forge/30 text-[#aebbd6]",
        quilt: "bg-loader-quilt/24 text-[#d79bf0]",
        neoforge: "bg-loader-neoforge/22 text-loader-neoforge",
        modrinth: "bg-src-modrinth/20 text-src-modrinth",
        curseforge: "bg-src-curseforge/20 text-src-curseforge",
      },
    },
    defaultVariants: { tone: "neutral" },
  },
);

export type BadgeTone = NonNullable<VariantProps<typeof badgeVariants>["tone"]>;

const DOTS: Partial<Record<BadgeTone, string>> = {
  success: "bg-grass-500",
  warning: "bg-gold-500",
  danger: "bg-tnt-500",
  info: "bg-diamond-500",
  hearth: "bg-hearth-500",
};

/** Compact pixel label for status, versions, and mod-loaders/sources. */
function Badge({
  className,
  tone,
  dot = false,
  children,
  ...props
}: ComponentProps<"span"> & VariantProps<typeof badgeVariants> & { dot?: boolean }) {
  return (
    <span data-slot="badge" className={cn(badgeVariants({ tone }), className)} {...props}>
      {dot && (
        <span
          className={cn("size-1.75 shadow-outline-dark", DOTS[tone ?? "neutral"] ?? "bg-ink-400")}
        />
      )}
      {children}
    </span>
  );
}

export { Badge, badgeVariants };
