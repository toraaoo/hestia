import { Link } from "@tanstack/react-router";
import { motion } from "framer-motion";
import type { Instance } from "@/lib/types";
import { SNAP, riseVariants } from "@/lib/motion";
import { TILES } from "@/lib/tiles";
import { loaderTone } from "@/lib/format";
import { Badge } from "@/components/ui/Badge";
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
      whileHover={{ y: -3 }}
      transition={{ duration: 0.12, ease: SNAP }}
      className="group relative flex flex-col overflow-hidden rounded-lg bg-surface-2 shadow-card-rest transition-shadow duration-100 hover:shadow-card-hover"
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
        <div className="pointer-events-none absolute inset-0 z-2 flex items-center justify-center bg-ink-950/50 opacity-0 transition-opacity duration-100 group-hover:opacity-100">
          <button
            title="Play"
            onClick={() => onPlay(instance)}
            className="pointer-events-auto flex size-13 items-center justify-center rounded-full bg-grass-500 text-on-grass shadow-lg transition-[transform,filter] duration-100 ease-snap hover:scale-108 hover:brightness-108"
          >
            <PlayIcon size={22} weight="fill" />
          </button>
        </div>
      </div>
      <div className="flex flex-col gap-2 p-3">
        <span className="truncate text-sm font-semibold text-text-1">{instance.name}</span>
        <div className="flex items-center gap-1.5">
          <Badge tone={loaderTone(instance.loader)}>{instance.loader}</Badge>
          <Badge>{instance.version}</Badge>
        </div>
        <span className="text-xs text-text-3">
          {instance.lastPlayed === "Never" ? "Never played" : `Last played ${instance.lastPlayed}`}
        </span>
      </div>
    </motion.div>
  );
}
