import { WarningIcon, XIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { type Announcement, system } from '@/api';
import { Markdown } from '@/components/markdown';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import { m } from '@/paraglide/messages.js';
import {
  unread,
  useAnnouncements,
  useDismissAnnouncements,
} from '@/queries/announce';

/**
 * Severity picks the surface, and each severity gets exactly one — intrusiveness
 * tracking urgency, with no two surfaces saying the same thing:
 *
 * - `critical` → a dialog, once per id. It blocks, so it is reserved for
 *   something that costs the user if they miss it; dismissal is the daemon's
 *   `seen.json`, so "once" survives restarts and covers the CLI too.
 * - `warning` → a standing strip until dismissed. Still true, not blocking.
 * - `info` → the /news page and the sidebar badge only.
 */
export function CriticalAnnouncementDialog() {
  const announcements = useAnnouncements();
  const dismiss = useDismissAnnouncements();
  // A dismissal is a round trip; without this the dialog would re-render from
  // stale cache before the invalidation lands and flash back open.
  const [acknowledged, setAcknowledged] = useState<string[]>([]);

  const pending = unread(announcements.data).filter(
    (a) => a.severity === 'critical' && !acknowledged.includes(a.id),
  );
  const current = pending[0];
  if (!current) return null;

  const acknowledge = () => {
    setAcknowledged((ids) => [...ids, current.id]);
    dismiss.mutate([current.id]);
  };

  return (
    <AlertDialog open>
      <AlertDialogContent className="sm:max-w-2xl">
        <AlertDialogHeader>
          <AlertDialogTitle>{current.title}</AlertDialogTitle>
          {/* A critical notice is authored, not length-limited — cap it and let
              it scroll rather than let a long one grow past the viewport. */}
          <AlertDialogDescription
            render={<div />}
            className="max-h-[60vh] overflow-y-auto pr-1"
          >
            <Markdown>{current.body}</Markdown>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          {current.link && (
            <Button
              variant="outline"
              onClick={() => void system.openUrl(current.link)}
            >
              {m['news.read_more']()}
            </Button>
          )}
          <AlertDialogAction onClick={acknowledge}>
            {m['news.acknowledge']()}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

/**
 * The standing form: a warning that is still true but does not block. Sits
 * above the routed page so it spans the content column and never scrolls away.
 */
export function AnnouncementBanner() {
  const announcements = useAnnouncements();
  const dismiss = useDismissAnnouncements();

  const pending = unread(announcements.data).filter(
    (a) => a.severity === 'warning',
  );
  const current = pending[0];
  if (!current) return null;

  return (
    <div className="flex shrink-0 items-start gap-2.5 border-b border-amber/40 bg-amber/5 px-5 py-2.5">
      <WarningIcon className="mt-0.5 size-4 shrink-0 text-amber" />
      <div className="min-w-0 flex-1">
        <p className="text-xs leading-relaxed font-medium">{current.title}</p>
        <BannerSummary announcement={current} />
      </div>
      {pending.length > 1 && (
        <Link
          to="/news"
          className="text-xs text-muted-foreground underline underline-offset-2"
        >
          +{pending.length - 1}
        </Link>
      )}
      <Button
        variant="ghost"
        size="icon"
        className="size-6 shrink-0"
        aria-label={m['app.action.close']()}
        onClick={() => dismiss.mutate([current.id])}
      >
        <XIcon className="size-3.5" />
      </Button>
    </div>
  );
}

/**
 * One line of the body. The strip is chrome, not a reader — the full markdown
 * (images, lists, links) lives on /news, which the title links to.
 */
function BannerSummary({ announcement }: { announcement: Announcement }) {
  const firstLine = announcement.body.split('\n').find((l) => l.trim()) ?? '';
  return (
    <p className="truncate text-xs text-muted-foreground">
      {firstLine}{' '}
      <Link to="/news" className="underline underline-offset-2">
        {m['news.read_more']()}
      </Link>
    </p>
  );
}
