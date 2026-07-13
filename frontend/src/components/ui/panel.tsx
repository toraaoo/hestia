import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { type VariantProps, cva } from "class-variance-authority";
import { cn } from "@/lib/utils";

const panelVariants = cva("overflow-hidden rounded-lg", {
  variants: {
    variant: {
      flat: "bg-surface-2 shadow-card-flat",
      inset: "bg-surface-inset shadow-bevel-inset",
    },
  },
  defaultVariants: { variant: "flat" },
});

export type PanelVariant = NonNullable<VariantProps<typeof panelVariants>["variant"]>;

interface PanelProps
  extends Omit<ComponentPropsWithoutRef<"div">, "title">, VariantProps<typeof panelVariants> {
  as?: "div" | "aside" | "section";
  /** Renders the DS panel header bar (icon/label left, actions right). */
  title?: ReactNode;
  actions?: ReactNode;
}

/** Surface container per the DS Panel: a flat card or a pressed-in well. */
function Panel({
  variant,
  as: Tag = "div",
  title,
  actions,
  className,
  children,
  ...props
}: PanelProps) {
  return (
    <Tag data-slot="panel" className={cn(panelVariants({ variant }), className)} {...props}>
      {(title != null || actions != null) && (
        <div className="flex shrink-0 items-center gap-2.5 border-b border-border-2 bg-ink-950 px-3.5 py-2.5 text-xs font-semibold text-fg-3">
          <span className="flex flex-1 items-center gap-2.5 truncate">{title}</span>
          {actions}
        </div>
      )}
      {children}
    </Tag>
  );
}

export { Panel, panelVariants };
