import { useState } from "react";
import { getRouteApi } from "@tanstack/react-router";
import { useContentSearch, useInstances } from "@/data";
import type { Loader } from "@/lib/types";
import { TopBar } from "@/components/layout/top-bar";
import { SearchField } from "@/components/ui/search-field";
import { Overline } from "@/components/ui/overline";
import { Panel } from "@/components/ui/panel";
import { TabButton, Tabs } from "@/components/ui/tabs";
import { Tile } from "@/components/ui/tile";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { CaretDownIcon } from "@/components/icons";
import { ProjectRow } from "./project-row";
import type { ContentKind } from "./tabs";

const route = getRouteApi("/discover");

const TABS = [
  { id: "mods", label: "Mods", count: 1284 },
  { id: "modpacks", label: "Modpacks", count: 412 },
  { id: "resourcepacks", label: "Resource Packs", count: 806 },
  { id: "shaders", label: "Shaders", count: 91 },
] as const;

const FILTER_LOADERS: Loader[] = ["Fabric", "Quilt", "Forge", "NeoForge"];

export function DiscoverScreen() {
  const tab = route.useSearch({ select: (s) => s.tab ?? "mods" });
  const navigate = route.useNavigate();
  const [query, setQuery] = useState("");

  const setTab = (next: ContentKind) => void navigate({ search: { tab: next }, replace: true });

  const [loaders, setLoaders] = useState<Record<string, boolean>>({
    Fabric: true,
    Quilt: false,
    Forge: false,
    NeoForge: false,
  });
  const target = useInstances()[0];
  const results = useContentSearch(query);

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
          <Tabs className="mb-4">
            {TABS.map(({ id, label, count }) => (
              <TabButton
                key={id}
                label={label}
                count={count}
                active={tab === id}
                onClick={() => setTab(id)}
              />
            ))}
          </Tabs>

          <div className="flex items-start gap-5">
            <Panel as="aside" className="flex w-50 shrink-0 flex-col gap-5 p-4">
              <div className="flex flex-col gap-2.5">
                <Overline>Install to</Overline>
                {target && (
                  <button className="flex items-center gap-2.5 rounded-sm bg-surface-inset px-2.5 py-2 shadow-bevel-inset">
                    <Tile tile={target.tile} className="size-6 rounded-xs" />
                    <span className="text-sm font-semibold text-fg-1">{target.name}</span>
                    <CaretDownIcon size={14} className="ml-auto text-fg-3" />
                  </button>
                )}
              </div>
              <div className="flex flex-col gap-2.5">
                <Overline>Loaders</Overline>
                {FILTER_LOADERS.map((loader) => (
                  <FilterCheck
                    key={loader}
                    label={loader}
                    checked={loaders[loader] ?? false}
                    onCheckedChange={() => setLoaders((s) => ({ ...s, [loader]: !s[loader] }))}
                  />
                ))}
              </div>
              <div className="flex flex-col gap-2.5">
                <Overline>Source</Overline>
                {["Modrinth", "CurseForge"].map((source) => (
                  <FilterCheck key={source} label={source} defaultChecked />
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

/** A labelled filter checkbox (label click toggles — the Base UI root is a <button>). */
function FilterCheck({
  label,
  checked,
  defaultChecked,
  onCheckedChange,
}: {
  label: string;
  checked?: boolean;
  defaultChecked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
}) {
  const id = `filter-${label}`;
  return (
    <div className="flex items-center gap-2.5">
      <Checkbox
        id={id}
        checked={checked}
        defaultChecked={defaultChecked}
        onCheckedChange={onCheckedChange}
      />
      <Label htmlFor={id} className="font-normal text-fg-2">
        {label}
      </Label>
    </div>
  );
}
