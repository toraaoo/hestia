import { createFileRoute, notFound } from "@tanstack/react-router";
import { TILES } from "../../lib/tiles";
import { useLauncherStore } from "../../lib/store";
import { MOCK_SERVER_LOG } from "../../lib/mock";
import { LogLines } from "../../components/LogView";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { ProgressBar } from "../../components/ui/ProgressBar";
import { CloseIcon, MenuIcon, PlayIcon, ReloadIcon } from "../../components/icons";

export const Route = createFileRoute("/servers/$serverId")({
  component: ServerDetail,
});

function ServerDetail() {
  const { serverId } = Route.useParams();
  const server = useLauncherStore((s) => s.servers.find((x) => x.id === serverId));
  const isUp = useLauncherStore((s) => s.serverRunning[serverId] ?? false);
  const setRunning = useLauncherStore((s) => s.setServerRunning);

  // eslint-disable-next-line @typescript-eslint/only-throw-error -- the router catches its own non-Error marker
  if (!server) throw notFound();

  return (
    <section className="flex min-w-0 flex-1 flex-col gap-3.5">
      <div className="flex items-center gap-3.5">
        <img src={TILES[server.tile]} alt="" className="size-13 rounded-lg shadow-tile pixelated" />
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
        <Stat label="Players" value={`${isUp ? server.players : 0}/${server.maxPlayers}`} />
        <Stat label="TPS" value={isUp ? server.tps.toFixed(1) : "—"} accent={isUp} />
        <Stat label="Uptime" value={isUp ? server.uptime : "—"} />
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

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg bg-surface-inset shadow-bevel-inset">
        <div className="flex items-center gap-2.5 border-b border-border-2 bg-ink-950 px-3.5 py-2.5 text-xs font-semibold text-text-3">
          <MenuIcon size={13} />
          Console
          <div className="flex-1" />
          <button className="text-xs font-semibold text-text-3 hover:text-hearth-400">Clear</button>
        </div>
        <div className="min-h-25 flex-1 overflow-y-auto p-3.5 font-mono text-xs leading-relaxed">
          {isUp ? (
            <LogLines lines={MOCK_SERVER_LOG} />
          ) : (
            <div className="font-body text-sm text-text-3">
              Server is stopped. Press Start to boot it up.
            </div>
          )}
        </div>
        <div className="flex h-11 items-center gap-2 border-t border-border-2 bg-ink-950 px-3.5">
          <span className="font-mono text-sm text-hearth-400">&gt;</span>
          <input
            disabled={!isUp}
            placeholder={isUp ? "Type a command (e.g. /weather clear)…" : "Server offline"}
            className="flex-1 bg-transparent font-mono text-xs text-text-1 outline-none placeholder:text-text-3"
          />
        </div>
      </div>
    </section>
  );
}

function Stat({
  label,
  value,
  accent = false,
}: {
  label: string;
  value: string;
  accent?: boolean;
}) {
  return (
    <div className="flex min-w-23 shrink-0 flex-col gap-1 rounded-lg bg-surface-2 px-4 py-3 shadow-card-flat">
      <span className={`font-hero text-lg font-crisp ${accent ? "text-grass-400" : "text-text-1"}`}>
        {value}
      </span>
      <span className="text-xs text-text-3">{label}</span>
    </div>
  );
}
