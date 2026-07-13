import { useNavigate } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { useInstances, usePlay, useSelectedInstance } from "@/data";
import { loaderTone } from "@/lib/format";
import { cn } from "@/lib/cn";
import { SNAP } from "@/lib/motion";
import { useLauncherStore } from "@/stores/launcher";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { StatusDot } from "@/components/ui/status-dot";
import { Tile } from "@/components/ui/tile";
import { CaretUpIcon, CheckIcon, PlayIcon } from "@/components/icons";

/** Slim selected-instance play bar pinned under the content column. */
export function PlayBar() {
  const instance = useSelectedInstance();
  const play = usePlay();
  const navigate = useNavigate();

  if (!instance) return null;

  const open = () =>
    void navigate({ to: "/instance/$instanceId", params: { instanceId: instance.id } });

  return (
    <motion.div
      initial={{ y: 24, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ duration: 0.26, ease: SNAP }}
      className="flex h-18 shrink-0 items-center gap-sm border-t border-border-1 bg-chrome pr-5 pl-4"
    >
      <InstancePicker selectedId={instance.id} />
      <div className="flex-1" />
      <div className="mr-1.5 flex items-center gap-2">
        <Badge tone={loaderTone(instance.loader)}>{instance.loader}</Badge>
        <Badge>{instance.version}</Badge>
      </div>
      <Button variant="ghost" onClick={open}>
        Manage
      </Button>
      <Button variant="play" size="lg" onClick={() => play(instance)} className="min-w-45 px-8">
        <PlayIcon size={16} weight="fill" />
        PLAY
      </Button>
    </motion.div>
  );
}

/** Selected-instance trigger + menu that swaps which instance the bar targets. */
function InstancePicker({ selectedId }: { selectedId: string }) {
  const instances = useInstances();
  const select = useLauncherStore((s) => s.select);
  const current = instances.find((i) => i.id === selectedId);

  if (!current) return null;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger className="group flex w-64 items-center gap-3 rounded-md py-1 pr-2 pl-1 outline-hidden transition-colors duration-100 ease-snap hover:bg-surface-hover">
        <Tile tile={current.tile} className="size-11" />
        <span className="flex min-w-0 flex-1 flex-col gap-0.5 text-left">
          <span className="text-xs text-fg-3">Selected</span>
          <span className="truncate text-base font-bold text-fg-1">{current.name}</span>
        </span>
        <CaretUpIcon
          size={14}
          className={cn(
            "shrink-0 rotate-180 text-fg-3 transition-transform duration-150 ease-snap",
            "group-data-[popup-open]:rotate-0",
          )}
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent side="top" align="start">
        <DropdownMenuGroup>
          <DropdownMenuLabel>Instances</DropdownMenuLabel>
          {instances.map((inst) => (
            <DropdownMenuItem
              key={inst.id}
              onClick={() => select(inst.id)}
              className="gap-2.5 py-1.5"
            >
              <Tile tile={inst.tile} className="size-8" />
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="truncate text-sm font-medium text-fg-1">{inst.name}</span>
                <span className="text-xs text-fg-3">
                  {inst.loader} · {inst.version}
                </span>
              </span>
              {inst.running && <StatusDot on size="sm" />}
              {inst.id === selectedId && <CheckIcon size={15} className="text-hearth-400" />}
            </DropdownMenuItem>
          ))}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
