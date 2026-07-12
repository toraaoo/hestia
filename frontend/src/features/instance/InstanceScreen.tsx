import { useEffect, useState } from "react";
import { Link, getRouteApi } from "@tanstack/react-router";
import type { Instance } from "@/lib/types";
import { orNotFound } from "@/lib/router";
import { useInstance, usePlay } from "@/data";
import { useLauncherStore } from "@/stores/launcher";
import { IconButton } from "@/components/ui/Button";
import { Tabs, type TabItem } from "@/components/ui/Tabs";
import { ArrowLeftIcon } from "@/components/icons";
import { type InstanceTab } from "./tabs";
import { Hero } from "./Hero";
import { OverviewTab } from "./OverviewTab";
import { ModsTab } from "./ModsTab";
import { WorldsTab } from "./WorldsTab";
import { ScreenshotsTab } from "./ScreenshotsTab";
import { LogsTab } from "./LogsTab";
import { SettingsTab } from "./SettingsTab";

const route = getRouteApi("/instance/$instanceId");

function instanceTabs(instance: Instance): TabItem<InstanceTab>[] {
  return [
    { id: "overview", label: "Overview" },
    { id: "mods", label: "Mods", count: instance.modCount },
    { id: "worlds", label: "Worlds", count: instance.worldCount },
    { id: "screenshots", label: "Screenshots" },
    { id: "logs", label: "Logs" },
    { id: "settings", label: "Settings" },
  ];
}

export function InstanceScreen() {
  const { instanceId } = route.useParams();
  const instance = orNotFound(useInstance(instanceId));
  const select = useLauncherStore((s) => s.select);
  const play = usePlay();
  const [tab, setTab] = useState<InstanceTab>("overview");

  useEffect(() => {
    select(instance.id);
  }, [instance, select]);

  return (
    <>
      <div className="flex h-13 shrink-0 items-center gap-3 border-b border-border-2 bg-app px-6">
        <Link to="/">
          <IconButton quiet title="Back">
            <ArrowLeftIcon size={18} />
          </IconButton>
        </Link>
        <span className="font-hero text-base text-text-1 font-crisp">
          Library / {instance.name}
        </span>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto">
        <Hero instance={instance} onPlay={play} />

        <Tabs
          items={instanceTabs(instance)}
          value={tab}
          onChange={setTab}
          className="mt-3.5 px-6 pt-4"
        />

        <div className="px-6 pt-5 pb-10">
          {tab === "overview" && <OverviewTab instance={instance} />}
          {tab === "mods" && <ModsTab instance={instance} />}
          {tab === "worlds" && <WorldsTab instance={instance} />}
          {tab === "screenshots" && <ScreenshotsTab />}
          {tab === "logs" && <LogsTab instance={instance} />}
          {tab === "settings" && <SettingsTab instance={instance} />}
        </div>
      </div>
    </>
  );
}
