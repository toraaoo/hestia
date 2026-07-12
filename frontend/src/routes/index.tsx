import { useState } from "react";
import { Link, createFileRoute } from "@tanstack/react-router";
import type { Instance } from "../lib/types";
import { TILES } from "../lib/tiles";
import { useLauncherStore } from "../lib/store";
import { TopBar } from "../components/TopBar";
import { SearchField } from "../components/ui/SearchField";
import { Badge } from "../components/ui/Badge";
import { loaderTone } from "../lib/format";
import { Button } from "../components/ui/Button";
import { CaretRightIcon, GridIcon, PlayIcon, PlusIcon, ViewListIcon } from "../components/icons";

export const Route = createFileRoute("/")({
  component: Library,
});

type LoaderFilter = "all" | "fabric" | "forge";

function Library() {
  const instances = useLauncherStore((s) => s.instances);
  const view = useLauncherStore((s) => s.libraryView);
  const setView = useLauncherStore((s) => s.setLibraryView);
  const play = useLauncherStore((s) => s.play);
  const servers = useLauncherStore((s) => s.servers);
  const serverRunning = useLauncherStore((s) => s.serverRunning);

  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<LoaderFilter>("all");

  let list = instances;
  if (filter === "fabric") list = list.filter((i) => i.loader === "Fabric");
  if (filter === "forge")
    list = list.filter((i) => i.loader === "Forge" || i.loader === "NeoForge");
  if (query) list = list.filter((i) => i.name.toLowerCase().includes(query.toLowerCase()));

  const onlineServers = servers.filter((s) => serverRunning[s.id]);

  return (
    <>
      <TopBar title="Library">
        <SearchField value={query} onChange={setQuery} placeholder="Search your instances" />
        <div className="flex gap-0.5 rounded-sm bg-surface-2 p-0.75 shadow-card-flat">
          {(
            [
              ["grid", "Grid", GridIcon],
              ["list", "List", ViewListIcon],
            ] as const
          ).map(([mode, title, ModeIcon]) => (
            <button
              key={mode}
              title={title}
              onClick={() => setView(mode)}
              className={`flex h-7.5 w-8 items-center justify-center rounded-xs ${
                view === mode ? "bg-surface-active text-text-1" : "text-text-3 hover:text-text-1"
              }`}
            >
              <ModeIcon size={15} />
            </button>
          ))}
        </div>
        <Link to="/discover">
          <Button variant="primary">
            <PlusIcon size={15} />
            New
          </Button>
        </Link>
      </TopBar>

      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="px-6 pt-5 pb-10">
          <SectionHeading
            title="Your Servers"
            action={
              <Link
                to="/servers"
                className="text-sm font-semibold text-hearth-400 hover:text-hearth-300"
              >
                Manage all
              </Link>
            }
          />
          <div className="flex flex-wrap gap-3">
            {onlineServers.map((server) => (
              <Link
                key={server.id}
                to="/servers/$serverId"
                params={{ serverId: server.id }}
                className="flex w-68 items-center gap-3 rounded-lg bg-surface-2 p-3 text-left shadow-card-rest transition-[box-shadow,transform] duration-100 ease-snap hover:-translate-y-0.5 hover:shadow-card-hover"
              >
                <img
                  src={TILES[server.tile]}
                  alt=""
                  className="size-9.5 rounded-sm shadow-tile pixelated"
                />
                <span className="flex min-w-0 flex-1 flex-col gap-1">
                  <span className="truncate text-sm font-semibold text-text-1">{server.name}</span>
                  <span className="flex items-center gap-2 text-xs text-text-3">
                    <Badge tone="success" dot>
                      Online
                    </Badge>
                    {server.players}/{server.maxPlayers} players
                  </span>
                </span>
                <CaretRightIcon size={16} className="text-text-3" />
              </Link>
            ))}
            <Link
              to="/servers"
              className="flex w-68 items-center justify-center gap-2 rounded-lg bg-surface-2 p-3 text-sm font-semibold text-text-3 shadow-card-rest transition-[box-shadow,transform] duration-100 ease-snap hover:-translate-y-0.5 hover:shadow-card-hover"
            >
              <PlusIcon size={16} />
              Host a server
            </Link>
          </div>

          <div className="mt-8 mb-3.5 flex items-center gap-3">
            <h2 className="font-hero text-base tracking-wide text-text-1 font-crisp">Instances</h2>
            <div className="ml-2 flex gap-1.5">
              {(["all", "fabric", "forge"] as const).map((f) => (
                <Button
                  key={f}
                  size="sm"
                  variant={filter === f ? "primary" : "ghost"}
                  className="capitalize"
                  onClick={() => setFilter(f)}
                >
                  {f === "all" ? `All ${instances.length}` : f}
                </Button>
              ))}
            </div>
          </div>

          {view === "grid" ? (
            <div className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-4">
              {list.map((inst) => (
                <InstanceCard key={inst.id} instance={inst} onPlay={play} />
              ))}
              <Link
                to="/discover"
                className="flex min-h-45 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-border-1 text-sm font-semibold text-text-3 transition-colors duration-100 ease-snap hover:border-hearth-500 hover:text-hearth-400"
              >
                <PlusIcon size={26} />
                New instance
              </Link>
            </div>
          ) : (
            <div className="flex flex-col gap-2">
              {list.map((inst) => (
                <InstanceRow key={inst.id} instance={inst} onPlay={play} />
              ))}
            </div>
          )}
        </div>
      </div>
    </>
  );
}

function SectionHeading({ title, action }: { title: string; action?: React.ReactNode }) {
  return (
    <div className="mt-1.5 mb-3.5 flex items-center gap-3">
      <h2 className="font-hero text-base tracking-wide text-text-1 font-crisp">{title}</h2>
      <div className="flex-1" />
      {action}
    </div>
  );
}

function InstanceCard({ instance, onPlay }: { instance: Instance; onPlay: (i: Instance) => void }) {
  return (
    <div className="group relative flex flex-col overflow-hidden rounded-lg bg-surface-2 shadow-card-rest transition-[box-shadow,transform] duration-100 ease-snap hover:-translate-y-0.75 hover:shadow-card-hover">
      <Link
        to="/instance/$instanceId"
        params={{ instanceId: instance.id }}
        aria-label={instance.name}
        className="absolute inset-0 z-1"
      />
      <div
        className="relative h-30 overflow-hidden bg-size-[26px_26px] pixelated"
        style={{ backgroundImage: `url(${TILES[instance.tile]})` }}
      >
        <div className="absolute inset-0 bg-gradient-to-b from-black/10 from-40% to-ink-900/75" />
        {instance.running && (
          <div className="absolute top-2 left-2">
            <Badge tone="success" dot>
              Running
            </Badge>
          </div>
        )}
        <div className="pointer-events-none absolute inset-0 z-2 flex items-center justify-center bg-ink-950/50 opacity-0 transition-opacity duration-100 group-hover:opacity-100">
          <button
            title="Play"
            onClick={() => onPlay(instance)}
            className="pointer-events-auto flex size-13 items-center justify-center rounded-full bg-grass-500 text-on-grass shadow-lg transition-[transform,filter] duration-100 ease-snap hover:scale-108 hover:brightness-108"
          >
            <PlayIcon size={22} weight="fill" />
          </button>
        </div>
      </div>
      <div className="flex flex-col gap-2 p-3">
        <span className="truncate text-sm font-semibold text-text-1">{instance.name}</span>
        <div className="flex items-center gap-1.5">
          <Badge tone={loaderTone(instance.loader)}>{instance.loader}</Badge>
          <Badge>{instance.version}</Badge>
        </div>
        <span className="text-xs text-text-3">
          {instance.lastPlayed === "Never" ? "Never played" : `Last played ${instance.lastPlayed}`}
        </span>
      </div>
    </div>
  );
}

function InstanceRow({ instance, onPlay }: { instance: Instance; onPlay: (i: Instance) => void }) {
  return (
    <div className="relative flex items-center gap-3.5 rounded-lg bg-surface-2 px-3.5 py-2.5 shadow-card-rest transition-shadow duration-100 hover:shadow-card-hover">
      <Link
        to="/instance/$instanceId"
        params={{ instanceId: instance.id }}
        aria-label={instance.name}
        className="absolute inset-0 z-1"
      />
      <img
        src={TILES[instance.tile]}
        alt=""
        className="size-11.5 rounded-sm object-cover shadow-tile pixelated"
      />
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="text-base font-semibold text-text-1">{instance.name}</span>
        <div className="flex items-center gap-2">
          <Badge tone={loaderTone(instance.loader)}>{instance.loader}</Badge>
          <Badge>{instance.version}</Badge>
          {instance.running && (
            <Badge tone="success" dot>
              Running
            </Badge>
          )}
          <span className="text-xs text-text-3">
            {instance.modCount} mods ·{" "}
            {instance.lastPlayed === "Never" ? "never played" : instance.lastPlayed}
          </span>
        </div>
      </div>
      <div className="z-2 flex items-center gap-2">
        <Link to="/instance/$instanceId" params={{ instanceId: instance.id }}>
          <Button variant="ghost" size="sm">
            Manage
          </Button>
        </Link>
        <Button variant="play" size="sm" onClick={() => onPlay(instance)}>
          <PlayIcon size={14} weight="fill" />
          Play
        </Button>
      </div>
    </div>
  );
}
