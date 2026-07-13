import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";
import { TILES } from "@/lib/tiles";
import type { TileName } from "@/lib/types";

interface TileProps extends ComponentProps<"img"> {
  tile: TileName;
  rounded?: "sm" | "lg";
}

/** Pixel-art entity thumbnail with the standard dark outline. */
function Tile({ tile, rounded = "sm", className, alt = "", ...props }: TileProps) {
  return (
    <img
      data-slot="tile"
      {...props}
      src={TILES[tile]}
      alt={alt}
      className={cn(
        "shrink-0 object-cover shadow-tile pixelated",
        rounded === "lg" ? "rounded-lg" : "rounded-sm",
        className,
      )}
    />
  );
}

export { Tile };
