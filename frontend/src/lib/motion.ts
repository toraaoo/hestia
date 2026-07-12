import type { Variants } from "framer-motion";

/** The `--ease-snap` design token, as a framer cubic-bezier array. */
export const SNAP: [number, number, number, number] = [0.2, 0.8, 0.2, 1];

/** Page content on route change: soft fade + rise. Enter-only by design —
    TanStack's live <Outlet/> renders the incoming route, so an exit animation
    would animate the new page out, not the old one. */
export const pageVariants: Variants = {
  initial: { opacity: 0, y: 10 },
  animate: { opacity: 1, y: 0, transition: { duration: 0.28, ease: SNAP } },
};

/** A card/row: fades up into place, staggered by its list index (via `custom`,
    capped so long lists don't crawl in). */
export const riseVariants: Variants = {
  initial: { opacity: 0, y: 8 },
  animate: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: { duration: 0.3, ease: SNAP, delay: Math.min(i, 12) * 0.03 },
  }),
};

/** Modal backdrop fade (paired with AnimatePresence). */
export const backdropVariants: Variants = {
  initial: { opacity: 0 },
  animate: { opacity: 1, transition: { duration: 0.16 } },
  exit: { opacity: 0, transition: { duration: 0.12 } },
};

/** Modal dialog pop (paired with AnimatePresence). */
export const popVariants: Variants = {
  initial: { opacity: 0, scale: 0.96 },
  animate: { opacity: 1, scale: 1, transition: { duration: 0.2, ease: SNAP } },
  exit: { opacity: 0, scale: 0.98, transition: { duration: 0.12, ease: SNAP } },
};
