import type { TileName } from "./types";

import tileDiamond from "../assets/tiles/tile-diamond.png";
import tileEnd from "../assets/tiles/tile-end.png";
import tileForge from "../assets/tiles/tile-forge.png";
import tileGrass from "../assets/tiles/tile-grass.png";
import tileNether from "../assets/tiles/tile-nether.png";
import tileOcean from "../assets/tiles/tile-ocean.png";
import tileServer from "../assets/tiles/tile-server.png";
import tileSky from "../assets/tiles/tile-sky.png";

export const TILES: Record<TileName, string> = {
  "tile-diamond": tileDiamond,
  "tile-end": tileEnd,
  "tile-forge": tileForge,
  "tile-grass": tileGrass,
  "tile-nether": tileNether,
  "tile-ocean": tileOcean,
  "tile-server": tileServer,
  "tile-sky": tileSky,
};
