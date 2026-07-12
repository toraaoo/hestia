import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

interface SectionHeadingProps {
  title: ReactNode;
  as?: "h2" | "h3";
  /** Extras rendered beside the title (filter chips, counts). */
  children?: ReactNode;
  /** Right-aligned action (a link or button). */
  action?: ReactNode;
  className?: string;
}

/** Hero-font section title row with optional inline extras and a right action. */
export function SectionHeading({
  title,
  as: Tag = "h2",
  children,
  action,
  className = "",
}: SectionHeadingProps) {
  return (
    <div className={cn("mb-3.5 flex items-center gap-3", className)}>
      <Tag className="font-hero text-base tracking-wide text-text-1 font-crisp">{title}</Tag>
      {children}
      <div className="flex-1" />
      {action}
    </div>
  );
}
