import { Link } from "@tanstack/react-router";
import { motion } from "framer-motion";
import type { Instance } from "@/lib/types";
import { riseVariants } from "@/lib/motion";
import { TILES } from "@/lib/tiles";
import { loaderTone } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { PlayIcon } from "@/components/icons";

interface InstanceCardProps {
  instance: Instance;
  onPlay: (instance: Instance) => void;
  index: number;
}

export function InstanceCard({ instance, onPlay, index }: InstanceCardProps) {
  return (
    <motion.div
      variants={riseVariants}
      custom={index}
      initial="initial"
      animate="animate"
      className="group relative flex flex-col overflow-hidden rounded-sm bg-surface-2 shadow-outline-dark"
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
        <div className="pointer-events-none absolute inset-0 z-2 flex items-center justify-center opacity-0 transition-opacity duration-200 ease-soft group-hover:opacity-100">
          <button
            title="Play"
            onClick={() => onPlay(instance)}
            className="pointer-events-auto flex size-11 items-center justify-center rounded-sm bg-black/45 text-white backdrop-blur-sm transition-colors duration-200 ease-soft hover:bg-black/60"
          >
            <PlayIcon size={18} weight="fill" />
          </button>
        </div>
      </div>
      <div className="flex flex-col gap-2 p-3.5">
        <span className="truncate text-sm font-semibold text-fg-1">{instance.name}</span>
        <div className="flex items-center gap-1.5">
          <Badge tone={loaderTone(instance.loader)}>{instance.loader}</Badge>
          <Badge>{instance.version}</Badge>
        </div>
        <span className="text-xs text-fg-3">
          {instance.lastPlayed === "Never" ? "Never played" : `Last played ${instance.lastPlayed}`}
        </span>
      </div>
    </motion.div>
  );
}
