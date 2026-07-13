import { TILES } from "@/lib/tiles";

const SHOT_TILES = [
  "tile-sky",
  "tile-grass",
  "tile-ocean",
  "tile-nether",
  "tile-end",
  "tile-diamond",
] as const;

export function ScreenshotsTab() {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3">
      {SHOT_TILES.map((tile) => (
        <div
          key={tile}
          className="aspect-video overflow-hidden rounded-lg bg-size-[22px_22px] shadow-card-flat pixelated"
          style={{ backgroundImage: `url(${TILES[tile]})` }}
        />
      ))}
    </div>
  );
}
