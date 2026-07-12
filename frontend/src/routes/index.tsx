import { useState } from "react";
import { Link, createFileRoute } from "@tanstack/react-router";
import { motion } from "framer-motion";
import type { Instance } from "@/lib/types";
import { SNAP, riseVariants } from "@/lib/motion";
import { TILES } from "@/lib/tiles";
import { useLauncherStore } from "@/lib/store";
import { loaderTone } from "@/lib/format";
import { TopBar } from "@/components/TopBar";
import { SearchField } from "@/components/ui/SearchField";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { SectionHeading } from "@/components/ui/SectionHeading";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { Tile } from "@/components/ui/Tile";
import { CaretRightIcon, GridIcon, PlayIcon, PlusIcon, ViewListIcon } from "@/components/icons";

export const Route = createFileRoute("/")({
  component: Library,
});

type LoaderFilter = "all" | "fabric" | "forge";

const VIEW_OPTIONS = [
  { value: "grid", title: "Grid", icon: GridIcon },
  { value: "list", title: "List", icon: ViewListIcon },
] as const;

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
        <SegmentedControl options={VIEW_OPTIONS} value={view} onChange={setView} />
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
            className="mt-1.5"
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
            {onlineServers.map((server, i) => (
              <motion.div
                key={server.id}
                variants={riseVariants}
                custom={i}
                initial="initial"
                animate="animate"
              >
                <Link
                  to="/servers/$serverId"
                  params={{ serverId: server.id }}
                  className="flex w-68 items-center gap-3 rounded-lg bg-surface-2 p-3 text-left shadow-card-rest transition-[box-shadow,transform] duration-100 ease-snap hover:-translate-y-0.5 hover:shadow-card-hover"
                >
                  <Tile tile={server.tile} className="size-9.5" />
                  <span className="flex min-w-0 flex-1 flex-col gap-1">
                    <span className="truncate text-sm font-semibold text-text-1">
                      {server.name}
                    </span>
                    <span className="flex items-center gap-2 text-xs text-text-3">
                      <Badge tone="success" dot>
                        Online
                      </Badge>
                      {server.players}/{server.maxPlayers} players
                    </span>
                  </span>
                  <CaretRightIcon size={16} className="text-text-3" />
                </Link>
              </motion.div>
            ))}
            <Link
              to="/servers"
              className="flex w-68 items-center justify-center gap-2 rounded-lg bg-surface-2 p-3 text-sm font-semibold text-text-3 shadow-card-rest transition-[box-shadow,transform] duration-100 ease-snap hover:-translate-y-0.5 hover:shadow-card-hover"
            >
              <PlusIcon size={16} />
              Host a server
            </Link>
          </div>

          <SectionHeading title="Instances" className="mt-8">
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
          </SectionHeading>

          {view === "grid" ? (
            <div className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-4">
              {list.map((inst, i) => (
                <InstanceCard key={inst.id} instance={inst} onPlay={play} index={i} />
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
              {list.map((inst, i) => (
                <InstanceRow key={inst.id} instance={inst} onPlay={play} index={i} />
              ))}
            </div>
          )}
        </div>
      </div>
    </>
  );
}

function InstanceCard({
  instance,
  onPlay,
  index,
}: {
  instance: Instance;
  onPlay: (i: Instance) => void;
  index: number;
}) {
  return (
    <motion.div
      variants={riseVariants}
      custom={index}
      initial="initial"
      animate="animate"
      whileHover={{ y: -3 }}
      transition={{ duration: 0.12, ease: SNAP }}
      className="group relative flex flex-col overflow-hidden rounded-lg bg-surface-2 shadow-card-rest transition-shadow duration-100 hover:shadow-card-hover"
    >
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
    </motion.div>
  );
}

function InstanceRow({
  instance,
  onPlay,
  index,
}: {
  instance: Instance;
  onPlay: (i: Instance) => void;
  index: number;
}) {
  return (
    <motion.div
      variants={riseVariants}
      custom={index}
      initial="initial"
      animate="animate"
      className="relative flex items-center gap-3.5 rounded-lg bg-surface-2 px-3.5 py-2.5 shadow-card-rest transition-shadow duration-100 hover:shadow-card-hover"
    >
      <Link
        to="/instance/$instanceId"
        params={{ instanceId: instance.id }}
        aria-label={instance.name}
        className="absolute inset-0 z-1"
      />
      <Tile tile={instance.tile} className="size-11.5" />
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
    </motion.div>
  );
}
