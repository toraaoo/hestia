import { Link, Outlet, createFileRoute } from "@tanstack/react-router";
import { TILES } from "../../lib/tiles";
import { useLauncherStore } from "../../lib/store";
import { TopBar } from "../../components/TopBar";
import { Button } from "../../components/ui/Button";
import { PlusIcon } from "../../components/icons";

export const Route = createFileRoute("/servers")({
  component: ServersLayout,
});

/** Master-detail layout: the server rail persists, the detail is the child route. */
function ServersLayout() {
  const servers = useLauncherStore((s) => s.servers);
  const running = useLauncherStore((s) => s.serverRunning);

  return (
    <>
      <TopBar title="Servers" subtitle="Host & manage your own Minecraft servers">
        <Button variant="primary">
          <PlusIcon size={15} /> New Server
        </Button>
      </TopBar>

      <div className="flex min-h-0 flex-1 flex-col px-6 py-5">
        <div className="flex min-h-0 flex-1 items-stretch gap-4.5">
          <aside className="flex w-57.5 shrink-0 flex-col gap-1.5 overflow-y-auto rounded-lg bg-surface-2 p-2 shadow-card-flat">
            {servers.map((server) => (
              <Link
                key={server.id}
                to="/servers/$serverId"
                params={{ serverId: server.id }}
                className="flex w-full items-center gap-3 rounded-sm px-2.5 py-2 text-left transition-colors duration-100 ease-snap"
                activeProps={{ className: "bg-surface-3" }}
                inactiveProps={{ className: "hover:bg-surface-hover" }}
              >
                <img
                  src={TILES[server.tile]}
                  alt=""
                  className="size-8.5 rounded-sm shadow-tile pixelated"
                />
                <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <span className="truncate text-sm font-semibold text-text-1">{server.name}</span>
                  <span className="font-mono text-xs text-text-3">:{server.port}</span>
                </span>
                <span
                  className={`size-2.25 shrink-0 rounded-full ${
                    running[server.id]
                      ? "bg-grass-500 shadow-[0_0_7px_var(--color-grass-500)]"
                      : "bg-ink-500"
                  }`}
                />
              </Link>
            ))}
          </aside>

          <Outlet />
        </div>
      </div>
    </>
  );
}
