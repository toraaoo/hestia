import { useInstanceWorlds } from "@/data";
import { Button } from "@/components/ui/button";
import { Tile } from "@/components/ui/tile";
import { PlayIcon } from "@/components/icons";
import { useCurrentInstance } from "./current";

export function WorldsTab() {
  const instance = useCurrentInstance();
  const worlds = useInstanceWorlds(instance.id);

  return (
    <div className="flex flex-col gap-2">
      {worlds.map((world) => (
        <div
          key={world.name}
          className="flex items-center gap-3 rounded-sm bg-surface-2 p-3.5 shadow-outline-dark"
        >
          <Tile tile={world.tile} className="size-10" />
          <div className="min-w-0 flex-1">
            <div className="text-sm font-semibold text-fg-1">{world.name}</div>
            <div className="mt-0.5 text-xs text-fg-3">{world.summary}</div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm">
              Backup
            </Button>
            <Button variant="play" size="sm">
              <PlayIcon size={13} weight="fill" /> Play
            </Button>
          </div>
        </div>
      ))}
    </div>
  );
}
