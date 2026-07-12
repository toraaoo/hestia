import { Outlet } from "@tanstack/react-router";
import { useIsServerRunning, useSetServerRunning } from "@/data";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { ProgressBar } from "@/components/ui/ProgressBar";
import { Stat } from "@/components/ui/Stat";
import { Tile } from "@/components/ui/Tile";
import { CloseIcon, PlayIcon, ReloadIcon } from "@/components/icons";
import { useCurrentServer } from "./current";

/** Layout route for /servers/$serverId: header + live stats; sub-views are child routes. */
export function ServerDetailLayout() {
  const server = useCurrentServer();
  const isUp = useIsServerRunning(server.id);
  const setRunning = useSetServerRunning();

  return (
    <section className="flex min-w-0 flex-1 flex-col gap-3.5">
      <div className="flex items-center gap-3.5">
        <Tile tile={server.tile} rounded="lg" className="size-13" />
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          <div className="flex items-center gap-2.5">
            <h2 className="font-hero text-xl text-text-1 font-crisp">{server.name}</h2>
            <Badge tone={isUp ? "success" : "neutral"} dot>
              {isUp ? "Running" : "Stopped"}
            </Badge>
          </div>
          <span className="font-mono text-xs text-text-3">
            localhost:{server.port} · {server.version}
          </span>
        </div>
        <div className="flex gap-2">
          {isUp ? (
            <Button variant="danger" onClick={() => setRunning(server.id, false)}>
              <CloseIcon size={14} /> Stop
            </Button>
          ) : (
            <Button variant="play" onClick={() => setRunning(server.id, true)}>
              <PlayIcon size={14} weight="fill" /> Start
            </Button>
          )}
          <Button disabled={!isUp}>
            <ReloadIcon size={14} /> Restart
          </Button>
        </div>
      </div>

      <div className="flex items-stretch gap-3">
        <Stat
          label="Players"
          value={`${isUp ? server.players : 0}/${server.maxPlayers}`}
          className="min-w-23 shrink-0"
        />
        <Stat
          label="TPS"
          value={isUp ? server.tps.toFixed(1) : "—"}
          accent={isUp}
          className="min-w-23 shrink-0"
        />
        <Stat label="Uptime" value={isUp ? server.uptime : "—"} className="min-w-23 shrink-0" />
        <div className="flex flex-1 flex-col justify-center rounded-lg bg-surface-2 px-4 py-3 shadow-card-flat">
          <ProgressBar
            value={isUp ? server.ramGb : 0}
            max={server.ramMaxGb}
            size="sm"
            showPct={false}
            label={`Memory  ${isUp ? server.ramGb : 0} / ${server.ramMaxGb} GB`}
          />
        </div>
      </div>

      <Outlet />
    </section>
  );
}
