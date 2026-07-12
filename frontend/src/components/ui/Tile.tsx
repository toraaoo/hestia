import { cn } from "../../lib/cn";
import { TILES } from "../../lib/tiles";
import type { TileName } from "../../lib/types";

interface TileProps {
  tile: TileName;
  rounded?: "sm" | "lg";
  className?: string;
}

/** Pixel-art entity thumbnail with the standard dark outline. */
export function Tile({ tile, rounded = "sm", className = "" }: TileProps) {
  return (
    <img
      src={TILES[tile]}
      alt=""
      className={cn(
        "shrink-0 object-cover shadow-tile pixelated",
        rounded === "lg" ? "rounded-lg" : "rounded-sm",
        className,
      )}
    />
  );
}
