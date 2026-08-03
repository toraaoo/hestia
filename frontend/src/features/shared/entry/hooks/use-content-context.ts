import { createContext, useContext } from 'react';
import type { ContentHandlers, EntryTarget } from '../lib';

/**
 * What every row in a content list needs but none of the layers between it and
 * the section actually read — the entry it acts on, the mutation handlers, and
 * the two lookups a row resolves its labels against.
 */
export interface ContentContext {
  entry: EntryTarget;
  handlers: ContentHandlers;
  /** The name of the pack the entry runs, empty when it runs none. */
  packName: string;
  /** The entry's save worlds, for a datapack that names none of its own. */
  entryWorlds: string[];
  /**
   * A content job is in flight on this entry. The daemon admits one content
   * change per entry at a time, so a second one is refused rather than queued.
   */
  busy: boolean;
}

export const ContentCtx = createContext<ContentContext | null>(null);

export function useContent(): ContentContext {
  const ctx = useContext(ContentCtx);
  if (!ctx) throw new Error('useContent must be used within a ContentSection');
  return ctx;
}
