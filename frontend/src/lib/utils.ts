import { type ClassValue, clsx } from "clsx";
import { extendTailwindMerge } from "tailwind-merge";

/**
 * tailwind-merge defaults to lumping every `font-*` class into one font-family
 * group, so it would drop our custom `font-hero`/`font-pixel` when they sit
 * beside `font-crisp` (a standalone pixel-rendering utility that is NOT a
 * font-family). Pin the font-family set to the real families so `font-crisp`
 * and the pixel fonts stop clobbering each other.
 */
const twMerge = extendTailwindMerge({
  override: {
    classGroups: {
      "font-family": [{ font: ["hero", "pixel", "body", "mono", "sans", "serif"] }],
    },
  },
});

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
