/**
 * `announce.*` — the signed feed's cached entries. Dismissal is a flag rather
 * than a deletion, so the news page can show history and the badge can count
 * only what is unread.
 */
import type { AnnounceListResult, Announcement } from '@/api/types';

import { publish } from '../bus';
import * as settings from '../state/settings';
import { ago, type Handlers, now, strings } from '../support';

const announcements: Announcement[] = [
  {
    id: 'mock-2026-07-vertical-slice',
    severity: 'info',
    title: 'The vertical slice is complete',
    body: 'Servers, instances, content and skins all work end to end.\n\nThis entry is served by the **browser fixture daemon**.',
    link: 'https://example.net/hestia/news',
    published: ago(86_400 * 2),
    dismissed: false,
  },
  {
    id: 'mock-2026-06-curseforge',
    severity: 'warning',
    title: 'CurseForge needs an API key',
    body: 'Set `content.curseforge-key` before the source is offered.',
    link: '',
    published: ago(86_400 * 21),
    dismissed: true,
  },
];

let fetched = ago(1_800);

const result = (): AnnounceListResult => ({
  announcements,
  fetched,
  enabled: settings.enabled('announcements.enabled'),
});

const unread = (): number =>
  announcements.filter((entry) => !entry.dismissed).length;

export const channels: Handlers = {
  'announce.list': result,

  'announce.dismiss': (p) => {
    const ids = new Set(strings(p, 'ids'));
    for (const entry of announcements)
      if (ids.has(entry.id)) entry.dismissed = true;
    publish('announce.changed', { unread: unread() });
    return result();
  },

  // A refresh answers from cache when the fetch fails, so it never rejects —
  // only the timestamp moves.
  'announce.refresh': () => {
    fetched = now();
    publish('announce.changed', { unread: unread() });
    return result();
  },
};
