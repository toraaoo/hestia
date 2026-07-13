import type { ComponentProps, ReactNode } from "react";
import { cn } from "@/lib/utils";

interface SectionHeadingProps extends Omit<ComponentProps<"div">, "title"> {
  title: ReactNode;
  as?: "h2" | "h3";
  /** Right-aligned action (a link or button). */
  action?: ReactNode;
}

/** Hero-font section title row with optional inline extras and a right action. */
function SectionHeading({
  title,
  as: Tag = "h2",
  children,
  action,
  className,
  ...props
}: SectionHeadingProps) {
  return (
    <div
      data-slot="section-heading"
      className={cn("mb-3.5 flex items-center gap-3", className)}
      {...props}
    >
      <Tag className="font-hero text-base tracking-wide text-text-1 font-crisp">{title}</Tag>
      {children}
      <div className="flex-1" />
      {action}
    </div>
  );
}

export { SectionHeading };
