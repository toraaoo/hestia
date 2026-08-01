import { useState } from 'react';

/**
 * An editable copy of a daemon-owned value that re-seeds when the daemon's own
 * changes. `signature` is what identifies the source: while it holds, the draft
 * is the user's to edit; when it changes the draft is replaced. Adjusted during
 * render rather than in an effect, so an edit is never rendered against a stale
 * value first.
 */
export function useDraft<T>(
  value: T,
  signature: string,
): [T, (next: T) => void] {
  const [draft, setDraft] = useState(value);
  const [seededFrom, setSeededFrom] = useState(signature);

  if (seededFrom !== signature) {
    setSeededFrom(signature);
    setDraft(value);
  }

  return [draft, setDraft];
}
