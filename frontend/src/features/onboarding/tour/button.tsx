import { QuestionIcon } from '@phosphor-icons/react';
import { useEffect } from 'react';

import { Button } from '@/components/ui/button';
import { m } from '@/paraglide/messages.js';

import type { TourId } from './registry';
import { useTour } from './store';

/** `ready` gates the unprompted first run, so no step points at an unrendered element. */
export function TourButton({
  id,
  ready = true,
}: {
  id: TourId;
  ready?: boolean;
}) {
  const { start, seen, idle } = useTour();
  const unseen = !seen(id);

  useEffect(() => {
    if (ready && idle && unseen) start(id);
  }, [ready, idle, unseen, id, start]);

  return (
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={m['onboarding.tour.replay']()}
      onClick={() => start(id)}
    >
      <QuestionIcon />
    </Button>
  );
}
