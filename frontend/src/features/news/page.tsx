import {
  ArrowClockwiseIcon,
  ArrowSquareOutIcon,
  ChecksIcon,
} from '@phosphor-icons/react';
import { toast } from 'sonner';

import { type Announcement, type Severity, system } from '@/api';
import { Empty } from '@/components/empty';
import { Markdown } from '@/components/markdown';
import { Page } from '@/components/page';
import { Bone } from '@/components/skeleton';
import { Button } from '@/components/ui/button';
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
      <Markdown className="mt-1">{announcement.body}</Markdown>
      {announcement.link && (
        <Button
          variant="link"
          size="sm"
          className="mt-1 px-0"
          onClick={() => void system.openUrl(announcement.link)}
        >
          {m['news.read_more']()}
          <ArrowSquareOutIcon className="size-3.5" />
        </Button>
      )}
    </article>
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
