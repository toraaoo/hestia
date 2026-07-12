import { motion } from "framer-motion";
import type { ContentProject } from "@/lib/types";
import { riseVariants } from "@/lib/motion";
import { TILES } from "@/lib/tiles";
import { formatCount, loaderTone } from "@/lib/format";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";

export function ProjectRow({ project, index }: { project: ContentProject; index: number }) {
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
