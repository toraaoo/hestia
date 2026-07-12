import { useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import type { ContentProject, Loader } from "../lib/types";
import { TILES } from "../lib/tiles";
import { MOCK_DISCOVER, MOCK_INSTANCES } from "../lib/mock";
import { formatCount, loaderTone } from "../lib/format";
import { TopBar } from "../components/TopBar";
import { SearchField } from "../components/ui/SearchField";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { CheckLabel } from "../components/ui/form";
import { CaretDownIcon } from "../components/icons";

export const Route = createFileRoute("/discover")({
  component: Discover,
});

const TABS = [
  ["mods", "Mods", 1284],
  ["modpacks", "Modpacks", 412],
  ["resourcepacks", "Resource Packs", 806],
  ["shaders", "Shaders", 91],
] as const;

const FILTER_LOADERS: Loader[] = ["Fabric", "Quilt", "Forge", "NeoForge"];

function Discover() {
  const [tab, setTab] = useState<(typeof TABS)[number][0]>("mods");
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
          <div className="mb-4 flex gap-0.5 border-b border-border-2">
            {TABS.map(([id, label, count]) => (
              <button
                key={id}
                onClick={() => setTab(id)}
                className={`relative h-9.5 px-3 text-sm font-semibold transition-colors duration-100 ${
                  tab === id ? "text-text-1" : "text-text-3 hover:text-text-1"
                }`}
              >
                {label}
                <span className="ml-1.5 text-xs font-medium text-text-3">{count}</span>
                {tab === id && (
                  <span className="absolute inset-x-1.5 -bottom-px h-0.75 rounded-t-xs bg-hearth-500" />
                )}
              </button>
            ))}
          </div>

          <div className="flex items-start gap-5">
            <aside className="flex w-50 shrink-0 flex-col gap-5 rounded-lg bg-surface-2 p-4 shadow-card-flat">
              <div className="flex flex-col gap-2.5">
                <span className="text-xs font-bold tracking-wider text-text-3 uppercase">
                  Install to
                </span>
                {target && (
                  <button className="flex items-center gap-2.5 rounded-sm bg-surface-inset px-2.5 py-2 shadow-bevel-inset">
                    <img src={TILES[target.tile]} alt="" className="size-6 rounded-xs pixelated" />
                    <span className="text-sm font-semibold text-text-1">{target.name}</span>
                    <CaretDownIcon size={14} className="ml-auto text-text-3" />
                  </button>
                )}
              </div>
              <div className="flex flex-col gap-2.5">
                <span className="text-xs font-bold tracking-wider text-text-3 uppercase">
                  Loaders
                </span>
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
                <span className="text-xs font-bold tracking-wider text-text-3 uppercase">
                  Source
                </span>
                {["Modrinth", "CurseForge"].map((source) => (
                  <CheckLabel key={source} defaultChecked>
                    {source}
                  </CheckLabel>
                ))}
              </div>
            </aside>

            <div className="flex min-w-0 flex-1 flex-col gap-2.5">
              {results.map((project) => (
                <ProjectRow key={project.name} project={project} />
              ))}
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

function ProjectRow({ project }: { project: ContentProject }) {
  return (
    <div className="flex gap-3.5 rounded-sm bg-surface-2 p-3.5 shadow-outline-dark transition-colors duration-100 hover:bg-surface-hover">
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
    </div>
  );
}
