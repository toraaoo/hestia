import type { Transition, Variants } from 'motion/react';

export const EASE_OUT: [number, number, number, number] = [0.16, 1, 0.3, 1];

export const duration = {
  fast: 0.12,
  base: 0.18,
} as const;

export const layoutMorph: Transition = { duration: 0.24, ease: EASE_OUT };

export const spring = {
  track: { type: 'spring', stiffness: 180, damping: 30, mass: 0.7 },
} satisfies Record<string, Transition>;

const STAGGER_STEP = 0.025;
const STAGGER_WINDOW = 0.25;

export function listContainer(count: number): Variants {
  return {
    hidden: {},
    show: {
      transition: {
        staggerChildren: Math.min(
          STAGGER_STEP,
          STAGGER_WINDOW / Math.max(count, 1),
        ),
      },
    },
  };
}

export const listItem: Variants = {
  hidden: { opacity: 0, y: 4 },
  show: {
    opacity: 1,
    y: 0,
    transition: { duration: duration.base, ease: EASE_OUT },
  },
  exit: { opacity: 0, transition: { duration: duration.fast } },
};
