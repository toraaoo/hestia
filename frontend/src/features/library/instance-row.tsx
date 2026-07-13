import { Link } from "@tanstack/react-router";
import { motion } from "framer-motion";
import type { Instance } from "@/lib/types";
import { riseVariants } from "@/lib/motion";
import { loaderTone } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tile } from "@/components/ui/tile";
import { PlayIcon } from "@/components/icons";

interface InstanceRowProps {
  instance: Instance;
  onPlay: (instance: Instance) => void;
  index: number;
}

export function InstanceRow({ instance, onPlay, index }: InstanceRowProps) {
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
