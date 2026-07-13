import type { ComponentProps, ComponentPropsWithRef } from "react";
import { createLink } from "@tanstack/react-router";
import { cn } from "@/lib/utils";

/** Horizontal underline tab bar; children are TabLink (routes) or TabButton (filters). */
export function Tabs({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      role="tablist"
      data-slot="tabs"
      className={cn("flex items-center gap-0.5 border-b border-border-2", className)}
      {...props}
    />
  );
}

const TAB_CLASS =
  "relative flex h-10 items-center px-3.5 text-sm font-semibold transition-colors duration-100";

const UNDERLINE_CLASS = "absolute inset-x-2 -bottom-px h-0.75 rounded-t-xs bg-hearth-500";

function TabCount({ count }: { count?: number }) {
  if (count == null) return null;
  return <span className="ml-1.5 text-xs font-medium text-fg-3">{count}</span>;
}

interface TabAnchorProps extends ComponentPropsWithRef<"a"> {
  label: string;
  count?: number;
}

/* The router stamps data-status="active" on the matched link; CSS drives the state. */
function TabAnchor({ label, count, className: _ignored, ...rest }: TabAnchorProps) {
  return (
    <a
      {...rest}
      role="tab"
      data-slot="tab"
      className={cn(TAB_CLASS, "group text-fg-3 hover:text-fg-1 data-[status=active]:text-fg-1")}
    >
      {label}
      <TabCount count={count} />
      <span className={cn(UNDERLINE_CLASS, "hidden group-data-[status=active]:block")} />
    </a>
  );
}

/** A tab that is a child route: the router drives the active state. */
export const TabLink = createLink(TabAnchor);

interface TabButtonProps {
  label: string;
  count?: number;
  active: boolean;
  onClick: () => void;
}

/** A tab that filters in place (search-param tabs). */
export function TabButton({ label, count, active, onClick }: TabButtonProps) {
  return (
    <button
      type="button"
      role="tab"
      data-slot="tab"
      aria-selected={active}
      onClick={onClick}
      className={cn(TAB_CLASS, active ? "text-fg-1" : "text-fg-3 hover:text-fg-1")}
    >
      {label}
      <TabCount count={count} />
      {active && <span className={UNDERLINE_CLASS} />}
    </button>
  );
}
