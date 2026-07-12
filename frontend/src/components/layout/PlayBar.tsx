import { useNavigate } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { usePlay, useSelectedInstance } from "@/data";
import { loaderTone } from "@/lib/format";
import { SNAP } from "@/lib/motion";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { PlayButton } from "@/components/ui/PlayButton";
import { Tile } from "@/components/ui/Tile";

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
      className="flex h-18 shrink-0 items-center gap-4 border-t border-border-1 bg-chrome pr-5 pl-4"
    >
      <button onClick={open} className="shrink-0">
        <Tile tile={instance.tile} className="size-11" />
      </button>
      <div className="flex min-w-0 flex-col gap-0.5">
        <span className="text-xs text-text-3">Selected</span>
        <span className="text-base font-bold whitespace-nowrap text-text-1">{instance.name}</span>
      </div>
      <div className="flex-1" />
      <div className="mr-1.5 flex items-center gap-2">
        <Badge tone={loaderTone(instance.loader)}>{instance.loader}</Badge>
        <Badge>{instance.version}</Badge>
      </div>
      <Button variant="ghost" onClick={open}>
        Manage
      </Button>
      <PlayButton onClick={() => play(instance)} className="min-w-45 px-8" />
    </motion.div>
  );
}
