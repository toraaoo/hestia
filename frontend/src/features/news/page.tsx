import {
  ArrowClockwiseIcon,
  ArrowSquareOutIcon,
  ChecksIcon,
} from '@phosphor-icons/react';
import { useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';

import { type Announcement, type Severity, system } from '@/api';
import { Empty } from '@/components/empty';
import { Markdown } from '@/components/markdown';
import { Page } from '@/components/page';
import { Bone } from '@/components/skeleton';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { agoLabel } from '@/lib/format';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import {
  unread,
  useAnnouncements,
  useDismissAnnouncements,
  useRefreshAnnouncements,
} from '@/queries/announce';

export function severityLabel(severity: Severity): string {
  if (severity === 'critical') return m['domain.severity.critical']();
  if (severity === 'warning') return m['domain.severity.warning']();
  return m['domain.severity.info']();
}

/** Severity drives the accent; `info` stays neutral so news is not an alarm. */
export function severityAccent(severity: Severity): string {
  if (severity === 'critical') return 'text-destructive border-destructive/40';
  if (severity === 'warning') return 'text-amber border-amber/40';
  return 'text-muted-foreground border-border';
}

export function NewsPage() {
  const announcements = useAnnouncements();
  const dismiss = useDismissAnnouncements();
  const refresh = useRefreshAnnouncements();

  const result = announcements.data;
  const pending = unread(result);

  const markAllRead = () => {
    dismiss.mutate(pending.map((a) => a.id));
  };

  const check = () => {
    refresh.mutate(undefined, {
      onSuccess: () => toast.success(m['news.refreshed']()),
    });
  };

  return (
    <Page
      title={m['news.title']()}
      subtitle={
        result && result.fetched > 0
          ? m['news.checked']({ ago: agoLabel(result.fetched) })
          : m['news.subtitle']()
      }
      loading={announcements.isLoading}
      skeleton={<NewsSkeleton />}
      actions={
        <>
          {pending.length > 0 && (
            <Button
              variant="ghost"
              size="sm"
              onClick={markAllRead}
              disabled={dismiss.isPending}
            >
              <ChecksIcon className="size-4" />
              {m['news.mark_all_read']()}
            </Button>
          )}
          <Button
            variant="outline"
            size="sm"
            onClick={check}
            disabled={refresh.isPending || result?.enabled === false}
          >
            <ArrowClockwiseIcon
              className={cn('size-4', refresh.isPending && 'animate-spin')}
            />
            {m['news.refresh']()}
          </Button>
        </>
      }
    >
      {result?.enabled === false ? (
        <Empty>{m['news.disabled_body']()}</Empty>
      ) : result && result.announcements.length === 0 ? (
        <Empty>
          {result.fetched === 0
            ? m['news.never_fetched']()
            : m['news.empty_body']()}
        </Empty>
      ) : (
        <div className="space-y-3">
          {result?.announcements.map((announcement) => (
            <AnnouncementCard
              key={announcement.id}
              announcement={announcement}
              onRead={() => dismiss.mutate([announcement.id])}
            />
          ))}
        </div>
      )}
    </Page>
  );
}

function AnnouncementCard({
  announcement,
  onRead,
}: {
  announcement: Announcement;
  onRead: () => void;
}) {
  const { severity, dismissed } = announcement;
  return (
    <article
      className={cn(
        'border px-4 py-3',
        severityAccent(severity).split(' ')[1],
        dismissed && 'opacity-60',
      )}
    >
      <header className="flex items-center gap-2">
        {!dismissed && (
          <span
            className={cn(
              'size-1.5 shrink-0 rounded-full',
              severity === 'critical' ? 'bg-destructive' : 'bg-ember',
            )}
          />
        )}
        <span
          className={cn(
            'text-[0.7rem] font-semibold tracking-wide uppercase',
            severityAccent(severity).split(' ')[0],
          )}
        >
          {severityLabel(severity)}
        </span>
        <span className="text-xs text-muted-foreground">
          {agoLabel(announcement.published)}
        </span>
        {!dismissed && (
          <Button
            variant="ghost"
            size="sm"
            className="ml-auto"
            onClick={onRead}
          >
            {m['news.acknowledge']()}
          </Button>
        )}
      </header>
      <h2 className="mt-1 font-heading text-sm font-semibold">
        {announcement.title}
      </h2>
      <AnnouncementBody announcement={announcement} />
    </article>
  );
}

/** How much of a body the feed shows before it is worth opening on its own. */
const CLAMP_PX = 180;

/**
 * The feed is a list of announcements, not a reader: an entry is clamped to a
 * glance and opens in full on demand. The clamp is measured rather than guessed
 * from the body's length, because what overflows depends on what the markdown
 * renders to — a short body with a table or an image is taller than a long
 * paragraph.
 */
function AnnouncementBody({ announcement }: { announcement: Announcement }) {
  const [open, setOpen] = useState(false);
  const [overflows, setOverflows] = useState(false);
  const body = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = body.current;
    if (!el) return;
    // The parent clips, so the observed element keeps its natural height and
    // the measurement stays stable once clamped. Images and fonts settle late,
    // hence the observer rather than a single pass.
    const measure = () => setOverflows(el.scrollHeight > CLAMP_PX);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const openLink = () => void system.openUrl(announcement.link);

  return (
    <>
      <div
        className="relative mt-1 overflow-hidden"
        style={overflows ? { maxHeight: CLAMP_PX } : undefined}
      >
        <div ref={body}>
          <Markdown>{announcement.body}</Markdown>
        </div>
        {overflows && (
          <div className="pointer-events-none absolute inset-x-0 bottom-0 h-12 bg-gradient-to-t from-background to-transparent" />
        )}
      </div>

      <div className="flex items-center gap-3">
        {overflows && (
          <Button
            variant="link"
            size="sm"
            className="mt-1 px-0"
            onClick={() => setOpen(true)}
          >
            {m['news.show_full']()}
          </Button>
        )}
        {announcement.link && (
          <Button
            variant="link"
            size="sm"
            className="mt-1 px-0"
            onClick={openLink}
          >
            {m['news.read_more']()}
            <ArrowSquareOutIcon className="size-3.5" />
          </Button>
        )}
      </div>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>{announcement.title}</DialogTitle>
          </DialogHeader>
          <div className="max-h-[70vh] overflow-y-auto pr-1">
            <Markdown>{announcement.body}</Markdown>
          </div>
          <DialogFooter showCloseButton>
            {announcement.link && (
              <Button variant="outline" onClick={openLink}>
                {m['news.read_more']()}
                <ArrowSquareOutIcon className="size-3.5" />
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

function NewsSkeleton() {
  return (
    <div className="space-y-3">
      {[0, 1, 2].map((i) => (
        <div key={i} className="space-y-2 border border-border px-4 py-3">
          <Bone className="h-3 w-24" />
          <Bone className="h-4 w-2/3" />
          <Bone className="h-3 w-full" />
          <Bone className="h-3 w-4/5" />
        </div>
      ))}
    </div>
  );
}
