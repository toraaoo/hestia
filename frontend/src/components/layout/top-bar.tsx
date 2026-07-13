import type { ReactNode } from "react";

interface TopBarProps {
  title: ReactNode;
  subtitle?: string;
  children?: ReactNode;
}

/** Per-screen header row: hero-font title left, actions right. 52px per the DS topbar token. */
export function TopBar({ title, subtitle, children }: TopBarProps) {
  return (
    <div className="flex h-13 shrink-0 items-center gap-3 border-b border-border-2 bg-app px-6">
      <span className="font-hero text-lg tracking-wide text-text-1 font-crisp">{title}</span>
      {subtitle && <span className="ml-1 text-sm text-text-3">{subtitle}</span>}
      <div className="flex-1" />
      {children}
    </div>
  );
}
