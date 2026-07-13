import { Link, Outlet } from "@tanstack/react-router";
import { useServerRunning, useServers } from "@/data";
import { TopBar } from "@/components/layout/top-bar";
import { Button } from "@/components/ui/button";
import { Panel } from "@/components/ui/panel";
import { StatusDot } from "@/components/ui/status-dot";
import { Tile } from "@/components/ui/tile";
import { PlusIcon } from "@/components/icons";

/** Master-detail layout: the server rail persists, the detail is the child route. */
export function ServersLayout() {
  const servers = useServers();
  const running = useServerRunning();

  return (
    <>
      <TopBar title="Servers" subtitle="Host & manage your own Minecraft servers">
        <Button variant="primary">
          <PlusIcon size={15} /> New Server
        </Button>
      </TopBar>

      <div className="flex min-h-0 flex-1 flex-col px-6 py-5">
        <div className="flex min-h-0 flex-1 items-stretch gap-4.5">
          <Panel as="aside" className="flex w-57.5 shrink-0 flex-col gap-1.5 overflow-y-auto p-2">
            {servers.map((server) => (
              <Link
                key={server.id}
                to="/servers/$serverId"
                params={{ serverId: server.id }}
                className="flex w-full items-center gap-3 rounded-sm px-2.5 py-2 text-left transition-colors duration-100 ease-snap"
                activeProps={{ className: "bg-surface-3" }}
                inactiveProps={{ className: "hover:bg-surface-hover" }}
              >
                <Tile tile={server.tile} className="size-8.5" />
                <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <span className="truncate text-sm font-semibold text-fg-1">{server.name}</span>
                  <span className="font-mono text-xs text-fg-3">:{server.port}</span>
                </span>
                <StatusDot on={running[server.id] ?? false} />
              </Link>
            ))}
          </Panel>

          <Outlet />
        </div>
      </div>
    </>
  );
}
