export interface Box {
  top: number;
  left: number;
  width: number;
  height: number;
}

export interface Size {
  width: number;
  height: number;
}

export type Side = 'top' | 'bottom' | 'left' | 'right' | 'center';

export interface Placement {
  top: number;
  left: number;
  side: Side;
}

export const SPOTLIGHT_PADDING = 6;

const GAP = 14;
const MARGIN = 16;
const ORDER: Exclude<Side, 'center'>[] = ['bottom', 'top', 'right', 'left'];

export function inflate(box: Box, by: number): Box {
  return {
    top: box.top - by,
    left: box.left - by,
    width: box.width + by * 2,
    height: box.height + by * 2,
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), Math.max(min, max));
}

function room(target: Box, card: Size, viewport: Size): Record<string, number> {
  return {
    bottom: viewport.height - (target.top + target.height) - GAP - card.height,
    top: target.top - GAP - card.height,
    right: viewport.width - (target.left + target.width) - GAP - card.width,
    left: target.left - GAP - card.width,
  };
}

export function placeCard(
  target: Box | null,
  card: Size,
  viewport: Size,
): Placement {
  if (!target) {
    return {
      top: (viewport.height - card.height) / 2,
      left: (viewport.width - card.width) / 2,
      side: 'center',
    };
  }

  const spot = inflate(target, SPOTLIGHT_PADDING);
  const space = room(spot, card, viewport);
  const side =
    ORDER.find((candidate) => space[candidate] >= MARGIN) ??
    ORDER.reduce((best, candidate) =>
      space[candidate] > space[best] ? candidate : best,
    );

  const maxTop = viewport.height - card.height - MARGIN;
  const maxLeft = viewport.width - card.width - MARGIN;

  if (side === 'top' || side === 'bottom') {
    const top =
      side === 'bottom'
        ? spot.top + spot.height + GAP
        : spot.top - GAP - card.height;
    return {
      top: clamp(top, MARGIN, maxTop),
      left: clamp(spot.left + spot.width / 2 - card.width / 2, MARGIN, maxLeft),
      side,
    };
  }

  const left =
    side === 'right'
      ? spot.left + spot.width + GAP
      : spot.left - GAP - card.width;
  return {
    top: clamp(spot.top + spot.height / 2 - card.height / 2, MARGIN, maxTop),
    left: clamp(left, MARGIN, maxLeft),
    side,
  };
}
