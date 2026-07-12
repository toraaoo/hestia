import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

export type PanelVariant = "flat" | "inset";

const VARIANTS: Record<PanelVariant, string> = {
  flat: "bg-surface-2 shadow-card-flat",
  inset: "bg-surface-inset shadow-bevel-inset",
};

interface PanelProps {
  variant?: PanelVariant;
  as?: "div" | "aside" | "section";
  /** Renders the DS panel header bar (icon/label left, actions right). */
  title?: ReactNode;
  actions?: ReactNode;
  className?: string;
  children: ReactNode;
}

/** Surface container per the DS Panel: a flat card or a pressed-in well. */
export function Panel({
  variant = "flat",
  as: Tag = "div",
  title,
  actions,
  className = "",
  children,
}: PanelProps) {
  return (
    <Tag className={cn("overflow-hidden rounded-lg", VARIANTS[variant], className)}>
      {(title != null || actions != null) && (
        <div className="flex shrink-0 items-center gap-2.5 border-b border-border-2 bg-ink-950 px-3.5 py-2.5 text-xs font-semibold text-text-3">
          <span className="flex flex-1 items-center gap-2.5 truncate">{title}</span>
          {actions}
        </div>
      )}
      {children}
    </Tag>
  );
}
