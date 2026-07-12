import { useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { motion } from "framer-motion";
import type { ContentProject, Loader } from "../lib/types";
import { riseVariants } from "../lib/motion";
import { TILES } from "../lib/tiles";
import { MOCK_DISCOVER, MOCK_INSTANCES } from "../lib/mock";
import { formatCount, loaderTone } from "../lib/format";
import { TopBar } from "../components/TopBar";
import { SearchField } from "../components/ui/SearchField";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { Overline } from "../components/ui/Overline";
import { Panel } from "../components/ui/Panel";
import { Tabs } from "../components/ui/Tabs";
import { Tile } from "../components/ui/Tile";
import { CheckLabel } from "../components/ui/form";
import { CaretDownIcon } from "../components/icons";

export const Route = createFileRoute("/discover")({
  component: Discover,
});

type ContentKind = "mods" | "modpacks" | "resourcepacks" | "shaders";

const TABS = [
  { id: "mods", label: "Mods", count: 1284 },
  { id: "modpacks", label: "Modpacks", count: 412 },
  { id: "resourcepacks", label: "Resource Packs", count: 806 },
  { id: "shaders", label: "Shaders", count: 91 },
] as const;

const FILTER_LOADERS: Loader[] = ["Fabric", "Quilt", "Forge", "NeoForge"];

function Discover() {
  const [tab, setTab] = useState<ContentKind>("mods");
  const [query, setQuery] = useState("");
  const [loaders, setLoaders] = useState<Record<string, boolean>>({
    Fabric: true,
    Quilt: false,
    Forge: false,
    NeoForge: false,
  });
  const target = MOCK_INSTANCES[0];

  const results = query
    ? MOCK_DISCOVER.filter((p) => p.name.toLowerCase().includes(query.toLowerCase()))
    : MOCK_DISCOVER;

  return (
    <>
      <TopBar title="Discover">
        <SearchField
          wide
          value={query}
          onChange={setQuery}
          placeholder="Search Modrinth & CurseForge"
        />
      </TopBar>

      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="px-6 pt-5 pb-10">
          <Tabs items={TABS} value={tab} onChange={setTab} className="mb-4" />

          <div className="flex items-start gap-5">
            <Panel as="aside" className="flex w-50 shrink-0 flex-col gap-5 p-4">
              <div className="flex flex-col gap-2.5">
                <Overline>Install to</Overline>
                {target && (
                  <button className="flex items-center gap-2.5 rounded-sm bg-surface-inset px-2.5 py-2 shadow-bevel-inset">
                    <Tile tile={target.tile} className="size-6 rounded-xs" />
                    <span className="text-sm font-semibold text-text-1">{target.name}</span>
                    <CaretDownIcon size={14} className="ml-auto text-text-3" />
                  </button>
                )}
              </div>
              <div className="flex flex-col gap-2.5">
                <Overline>Loaders</Overline>
                {FILTER_LOADERS.map((loader) => (
                  <CheckLabel
                    key={loader}
                    checked={loaders[loader] ?? false}
                    onChange={() => setLoaders((s) => ({ ...s, [loader]: !s[loader] }))}
                  >
                    {loader}
                  </CheckLabel>
                ))}
              </div>
              <div className="flex flex-col gap-2.5">
                <Overline>Source</Overline>
                {["Modrinth", "CurseForge"].map((source) => (
                  <CheckLabel key={source} defaultChecked>
                    {source}
                  </CheckLabel>
                ))}
              </div>
            </Panel>

            <div className="flex min-w-0 flex-1 flex-col gap-2.5">
              {results.map((project, i) => (
                <ProjectRow key={project.name} project={project} index={i} />
              ))}
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

function ProjectRow({ project, index }: { project: ContentProject; index: number }) {
  return (
    <motion.div
      variants={riseVariants}
      custom={index}
      initial="initial"
      animate="animate"
      className="flex gap-3.5 rounded-sm bg-surface-2 p-3.5 shadow-outline-dark transition-colors duration-100 hover:bg-surface-hover"
    >
      <div className="flex size-15 shrink-0 items-center justify-center overflow-hidden rounded-xs bg-surface-inset shadow-outline-dark">
        <img src={TILES[project.tile]} alt="" className="size-full object-cover pixelated" />
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-1.5">
        <div className="flex flex-wrap items-baseline gap-2">
          <span className="font-pixel text-sm leading-tight tracking-wide font-crisp">
            {project.name}
          </span>
          <span className="text-xs text-text-3">by {project.author}</span>
        </div>
        <div className="line-clamp-2 text-xs leading-normal text-text-2">{project.description}</div>
        <div className="mt-0.5 flex items-center gap-3.5 text-xs text-text-3">
          <span>⬇ {formatCount(project.downloads)}</span>
          {project.likes != null && <span>♥ {formatCount(project.likes)}</span>}
          <span className="capitalize">◆ {project.source}</span>
        </div>
      </div>
      <div className="flex flex-col items-end justify-between gap-2">
        <div className="flex flex-wrap justify-end gap-1.5">
          {project.loaders.map((loader) => (
            <Badge key={loader} tone={loaderTone(loader)}>
              {loader}
            </Badge>
          ))}
        </div>
        {project.installed ? (
          <Button size="sm" disabled>
            ✓ Installed
          </Button>
        ) : (
          <Button variant="primary" size="sm">
            Install
          </Button>
        )}
      </div>
    </motion.div>
  );
}
