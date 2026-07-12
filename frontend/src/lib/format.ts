import type { BadgeTone } from "../components/ui/Badge";
import type { Loader } from "./types";

export function formatCount(n: number): string {
  if (n >= 1e6) return (n / 1e6).toFixed(1).replace(/\.0$/, "") + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(1).replace(/\.0$/, "") + "K";
  return String(n);
}

export function loaderTone(loader: Loader): BadgeTone {
  switch (loader) {
    case "Fabric":
      return "fabric";
    case "Forge":
      return "forge";
    case "Quilt":
      return "quilt";
    case "NeoForge":
      return "neoforge";
    default:
      return "neutral";
  }
}
