import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { useInstances, usePlay, useServerRunning, useServers } from "@/data";
import { useLauncherStore } from "@/stores/launcher";
import { riseVariants } from "@/lib/motion";
import { TopBar } from "@/components/layout/top-bar";
import { SearchField } from "@/components/ui/search-field";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { SectionHeading } from "@/components/ui/section-heading";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Tile } from "@/components/ui/tile";
import { CaretRightIcon, GridIcon, PlusIcon, ViewListIcon } from "@/components/icons";
import { InstanceCard } from "./instance-card";
import { InstanceRow } from "./instance-row";

type LoaderFilter = "all" | "fabric" | "forge";

const VIEW_OPTIONS = [
  { value: "grid", title: "Grid", icon: GridIcon },
  { value: "list", title: "List", icon: ViewListIcon },
] as const;

export function LibraryScreen() {
  const instances = useInstances();
  const servers = useServers();
  const serverRunning = useServerRunning();
  const view = useLauncherStore((s) => s.libraryView);
  const setView = useLauncherStore((s) => s.setLibraryView);
  const play = usePlay();

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
                    <span className="truncate text-sm font-semibold text-fg-1">{server.name}</span>
                    <span className="flex items-center gap-2 text-xs text-fg-3">
                      <Badge tone="success" dot>
                        Online
                      </Badge>
                      {server.players}/{server.maxPlayers} players
                    </span>
                  </span>
                  <CaretRightIcon size={16} className="text-fg-3" />
                </Link>
              </motion.div>
            ))}
            <Link
              to="/servers"
              className="flex w-68 items-center justify-center gap-2 rounded-lg bg-surface-2 p-3 text-sm font-semibold text-fg-3 shadow-card-rest transition-[box-shadow,transform] duration-100 ease-snap hover:-translate-y-0.5 hover:shadow-card-hover"
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
                className="flex min-h-45 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-border-1 text-sm font-semibold text-fg-3 transition-colors duration-100 ease-snap hover:border-hearth-500 hover:text-hearth-400"
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
