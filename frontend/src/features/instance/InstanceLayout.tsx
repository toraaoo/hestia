import { useEffect } from "react";
import { Link, Outlet } from "@tanstack/react-router";
import { usePlay } from "@/data";
import { useLauncherStore } from "@/stores/launcher";
import { IconButton } from "@/components/ui/Button";
import { TabLink, Tabs } from "@/components/ui/Tabs";
import { ArrowLeftIcon } from "@/components/icons";
import { useCurrentInstance } from "./current";
import { Hero } from "./Hero";

/** Layout route for /instance/$instanceId: header, hero, tab bar; tabs are child routes. */
export function InstanceLayout() {
  const instance = useCurrentInstance();
  const select = useLauncherStore((s) => s.select);
  const play = usePlay();

  useEffect(() => {
    select(instance.id);
  }, [instance, select]);

  const params = { instanceId: instance.id };

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

        <Tabs className="mt-3.5 px-6 pt-4">
          <TabLink
            to="/instance/$instanceId"
            params={params}
            activeOptions={{ exact: true }}
            label="Overview"
          />
          <TabLink
            to="/instance/$instanceId/mods"
            params={params}
            label="Mods"
            count={instance.modCount}
          />
          <TabLink
            to="/instance/$instanceId/worlds"
            params={params}
            label="Worlds"
            count={instance.worldCount}
          />
          <TabLink to="/instance/$instanceId/screenshots" params={params} label="Screenshots" />
          <TabLink to="/instance/$instanceId/logs" params={params} label="Logs" />
          <TabLink to="/instance/$instanceId/settings" params={params} label="Settings" />
        </Tabs>

        <div className="px-6 pt-5 pb-10">
          <Outlet />
        </div>
      </div>
    </>
  );
}
