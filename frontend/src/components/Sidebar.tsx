import type { ComponentType } from "react";
import { Link } from "@tanstack/react-router";
import { useLauncherStore } from "@/lib/store";
import { MOCK_ACCOUNT } from "@/lib/mock";
import { Badge } from "@/components/ui/Badge";
import { StatusDot } from "@/components/ui/StatusDot";
import { Tile } from "@/components/ui/Tile";
import {
  CaretUpIcon,
  GridIcon,
  PlusIcon,
  SearchIcon,
  ServerIcon,
  SlidersIcon,
  UserIcon,
} from "@/components/icons";

interface Section {
  to: "/" | "/discover" | "/servers" | "/skins" | "/settings";
  label: string;
  icon: ComponentType<{ size?: number }>;
}

const SECTIONS: Section[] = [
  { to: "/", label: "Library", icon: GridIcon },
  { to: "/discover", label: "Discover", icon: SearchIcon },
  { to: "/servers", label: "Servers", icon: ServerIcon },
  { to: "/skins", label: "Skins", icon: UserIcon },
  { to: "/settings", label: "Settings", icon: SlidersIcon },
];

export function Sidebar() {
  const instances = useLauncherStore((s) => s.instances);
  const serverRunning = useLauncherStore((s) => s.serverRunning);
  const pinned = instances.filter((i) => i.pinned);
  const onlineServers = Object.values(serverRunning).filter(Boolean).length;

  return (
    <aside className="flex w-58 shrink-0 flex-col border-r border-border-2 bg-chrome">
      <nav className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto px-3 py-3">
        {SECTIONS.map(({ to, label, icon: SectionIcon }) => (
          <Link
            key={to}
            to={to}
            activeOptions={{ exact: to === "/" }}
            className="relative flex w-full items-center gap-3 rounded-sm px-2.5 py-2 text-left text-sm font-medium transition-colors duration-100 ease-snap"
            activeProps={{ className: "bg-surface-3 text-text-1" }}
            inactiveProps={{ className: "text-text-2 hover:bg-surface-hover hover:text-text-1" }}
          >
            {({ isActive }) => (
              <>
                {isActive && (
                  <span className="absolute inset-y-2 -left-3 w-0.75 rounded-r-xs bg-hearth-500 shadow-glow-accent" />
                )}
                <SectionIcon size={18} />
                <span>{label}</span>
                {to === "/servers" && onlineServers > 0 && (
                  <span className="ml-auto">
                    <Badge tone="success" dot>
                      {onlineServers}
                    </Badge>
                  </span>
                )}
              </>
            )}
          </Link>
        ))}

        <div className="flex items-center px-2.5 pt-4 pb-1.5 text-xs font-semibold tracking-wider text-text-3 uppercase">
          Pinned
          <Link
            to="/discover"
            title="New instance"
            className="ml-auto flex text-text-3 hover:text-hearth-400"
          >
            <PlusIcon size={14} />
          </Link>
        </div>

        {pinned.map((inst) => (
          <Link
            key={inst.id}
            to="/instance/$instanceId"
            params={{ instanceId: inst.id }}
            className="flex w-full items-center gap-2.5 rounded-sm px-2.5 py-2 text-left transition-colors duration-100 ease-snap"
            activeProps={{ className: "bg-surface-3" }}
            inactiveProps={{ className: "hover:bg-surface-hover" }}
          >
            <Tile tile={inst.tile} className="size-6.5" />
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="truncate text-sm font-medium text-text-1">{inst.name}</span>
              <span className="text-xs text-text-3">
                {inst.loader} · {inst.version}
              </span>
            </span>
            {inst.running && <StatusDot on size="sm" />}
          </Link>
        ))}
      </nav>

      <button className="mx-2.5 mb-3 flex items-center gap-2.5 rounded-lg bg-surface-2 p-3 shadow-card-flat transition-colors duration-100 hover:bg-surface-hover">
        <Tile tile="tile-grass" className="size-7.5" />
        <span className="flex min-w-0 flex-1 flex-col text-left">
          <span className="text-sm font-semibold text-text-1">{MOCK_ACCOUNT.name}</span>
          <span className="text-xs text-text-3">{MOCK_ACCOUNT.kind}</span>
        </span>
        <CaretUpIcon size={15} className="text-text-3" />
      </button>
    </aside>
  );
}
