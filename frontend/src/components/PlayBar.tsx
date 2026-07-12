import { useNavigate } from "@tanstack/react-router";
import { TILES } from "../lib/tiles";
import { useLauncherStore, useSelectedInstance } from "../lib/store";
import { Badge } from "./ui/Badge";
import { loaderTone } from "../lib/format";
import { Button } from "./ui/Button";
import { PlayIcon } from "./icons";

/** Slim selected-instance play bar pinned under the content column. */
export function PlayBar() {
  const instance = useSelectedInstance();
  const play = useLauncherStore((s) => s.play);
  const navigate = useNavigate();

  if (!instance) return null;

  const open = () =>
    void navigate({ to: "/instance/$instanceId", params: { instanceId: instance.id } });

  return (
    <div className="flex h-18 shrink-0 items-center gap-4 border-t border-border-1 bg-chrome pr-5 pl-4">
      <button onClick={open} className="shrink-0">
        <img
          src={TILES[instance.tile]}
          alt=""
          className="size-11 rounded-sm object-cover shadow-tile pixelated"
        />
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
      <button
        onClick={() => play(instance)}
        className="flex h-12 min-w-45 items-center justify-center gap-2.5 rounded-lg bg-grass-500 px-8 text-on-grass shadow-bevel-btn transition-[filter,transform] duration-100 ease-snap hover:brightness-108 active:translate-y-px"
      >
        <PlayIcon size={16} weight="fill" />
        <span className="font-hero text-lg tracking-wide font-crisp">PLAY</span>
      </button>
    </div>
  );
}
