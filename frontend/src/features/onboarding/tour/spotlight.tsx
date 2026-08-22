import { motion } from 'motion/react';
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';

import { Button } from '@/components/ui/button';
import { duration, EASE_OUT } from '@/lib/motion';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

import {
  inflate,
  type Placement,
  placeCard,
  SPOTLIGHT_PADDING,
} from './placement';
import { useAnchorRect } from './rect';
import type { TourStep } from './registry';
import { useTour } from './store';

const CARD_WIDTH = 312;
const SCRIM = 'rgb(0 0 0 / 0.55)';

export function Spotlight() {
  const tour = useTour();
  if (!tour.run || !tour.step) return null;
  return createPortal(
    <Layer key={tour.run.id} step={tour.step} />,
    document.body,
  );
}

function Layer({ step }: { step: TourStep }) {
  const { run, steps, go, next, back, stop } = useTour();
  const { rect, missing } = useAnchorRect(step.anchor);
  const card = useRef<HTMLDivElement>(null);
  const [placement, setPlacement] = useState<Placement | null>(null);

  const index = run?.index ?? 0;
  const last = index === steps.length - 1;

  useEffect(() => {
    if (missing) next();
  }, [missing, next]);

  useLayoutEffect(() => {
    const element = card.current;
    if (!element) return;
    const { height } = element.getBoundingClientRect();
    setPlacement(
      placeCard(
        rect,
        { width: CARD_WIDTH, height },
        { width: window.innerWidth, height: window.innerHeight },
      ),
    );
  }, [rect]);

  useEffect(() => {
    card.current?.focus();
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') stop();
      else if (event.key === 'ArrowRight') next();
      else if (event.key === 'ArrowLeft') back();
      else return;
      event.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [next, back, stop]);

  const hole = rect ? inflate(rect, SPOTLIGHT_PADDING) : null;

  return (
    <div className="fixed inset-0 z-60" role="presentation">
      {hole ? (
        <motion.div
          aria-hidden
          className="pointer-events-none absolute ring-1 ring-ember"
          initial={false}
          animate={{
            top: hole.top,
            left: hole.left,
            width: hole.width,
            height: hole.height,
          }}
          transition={{ duration: duration.base, ease: EASE_OUT }}
          style={{ boxShadow: `0 0 0 9999px ${SCRIM}` }}
        />
      ) : (
        <div
          aria-hidden
          className="absolute inset-0"
          style={{ background: SCRIM }}
        />
      )}

      <div
        ref={card}
        role="dialog"
        aria-modal="true"
        aria-labelledby="tour-step-title"
        tabIndex={-1}
        onKeyDown={(event) => event.stopPropagation()}
        className={cn(
          'absolute flex flex-col gap-3 bg-popover p-4 text-popover-foreground shadow-lg ring-1 ring-foreground/10 outline-none transition-opacity',
          placement ? 'opacity-100' : 'opacity-0',
        )}
        style={{
          width: CARD_WIDTH,
          top: placement?.top ?? 0,
          left: placement?.left ?? 0,
        }}
      >
        <div className="space-y-1.5">
          <h2 id="tour-step-title" className="text-sm font-semibold">
            {step.title()}
          </h2>
          <p className="text-xs leading-relaxed text-muted-foreground">
            {step.body()}
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Dots steps={steps} index={index} onPick={go} />
          <span className="ml-auto flex items-center gap-1.5">
            {index > 0 && (
              <Button variant="ghost" size="sm" onClick={back}>
                {m['onboarding.tour.back']()}
              </Button>
            )}
            <Button size="sm" onClick={last ? stop : next}>
              {last ? m['onboarding.tour.done']() : m['onboarding.tour.next']()}
            </Button>
          </span>
        </div>

        <Button
          variant="ghost"
          size="xs"
          className="absolute -top-9 right-0 text-white/70 hover:bg-white/10 hover:text-white"
          onClick={stop}
        >
          {m['onboarding.tour.skip']()}
        </Button>
      </div>
    </div>
  );
}

function Dots({
  steps,
  index,
  onPick,
}: {
  steps: readonly TourStep[];
  index: number;
  onPick: (index: number) => void;
}) {
  return (
    <div className="flex items-center gap-1.5">
      {steps.map((step, at) => (
        <button
          key={step.title()}
          type="button"
          aria-label={m['onboarding.tour.progress']({
            current: at + 1,
            total: steps.length,
          })}
          aria-current={at === index ? 'step' : undefined}
          onClick={() => onPick(at)}
          className={cn(
            'size-1.5 transition-colors',
            at === index
              ? 'bg-ember'
              : 'bg-muted-foreground/35 hover:bg-muted-foreground/60',
          )}
        />
      ))}
    </div>
  );
}
