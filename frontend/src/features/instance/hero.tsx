import type { Instance } from "@/lib/types";
import { TILES } from "@/lib/tiles";
import { loaderTone } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button, IconButton } from "@/components/ui/button";
import { Tile } from "@/components/ui/tile";
import { FolderIcon, PlayIcon } from "@/components/icons";

export function Hero({ instance, onPlay }: { instance: Instance; onPlay: (i: Instance) => void }) {
  return (
    <div className="relative flex items-end gap-sm px-6 pt-5">
      <div
        className="absolute inset-x-0 top-0 h-37.5 bg-size-[34px_34px] opacity-50 pixelated"
        style={{ backgroundImage: `url(${TILES[instance.tile]})` }}
      />
      <div className="absolute inset-x-0 top-0 h-37.5 bg-gradient-to-b from-ink-900/40 to-app" />
      <Tile
        tile={instance.tile}

        className="relative size-24 shadow-md shadow-outline-dark"
      />
      <div className="relative flex min-w-0 flex-1 flex-col gap-2.5 pb-1">
        <h1 className="font-hero text-3xl leading-none tracking-wide text-fg-1 font-crisp">
          {instance.name}
        </h1>
        <div className="flex items-center gap-2">
          <Badge tone={loaderTone(instance.loader)}>{instance.loader}</Badge>
          <Badge>{instance.version}</Badge>
          {instance.running && (
            <Badge tone="success" dot>
              Running
            </Badge>
          )}
        </div>
      </div>
      <div className="relative flex items-center gap-2.5 pb-1">
        <IconButton title="Open folder">
          <FolderIcon size={18} />
        </IconButton>
        <Button variant="play" size="lg" onClick={() => onPlay(instance)}>
          <PlayIcon size={16} weight="fill" />
          PLAY
        </Button>
      </div>
    </div>
  );
}
