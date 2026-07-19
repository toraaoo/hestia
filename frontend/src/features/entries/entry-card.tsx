import { PlayIcon, PowerIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';

import { entryIcon } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { Spinner } from '@/components/ui/spinner';
import { StatusDot } from '@/components/ui/status-dot';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

export interface EntryCardData {
  id: string;
  name: string;
  kind: 'instance' | 'server';
  flavor: string;
  version: string;
  running: boolean;
  ready: boolean;
  /** One-line footer: "Last played 2h ago" / ":25565 · 3 online". */
  subtitle: string;
  /** Wired quick actions; absent leaves the button inert (mock surfaces). */
  onStart?: () => void;
  onStop?: () => void;
  busy?: boolean;
}

function statusOf(entry: EntryCardData) {
  if (!entry.ready)
    return { tone: 'warn' as const, label: m['status.preparing']() };
  if (entry.running)
    return {
      tone: 'on' as const,
      label:
        entry.kind === 'server' ? m['status.online']() : m['status.running'](),
    };
  return null;
}

function detailTo(kind: 'instance' | 'server') {
  return kind === 'server' ? '/servers/$id' : '/instances/$id';
}

function StatusBadge({
  status,
}: {
  status: NonNullable<ReturnType<typeof statusOf>>;
}) {
  return (
    <Badge
      variant="secondary"
      className="gap-1.5 bg-background/80 backdrop-blur-xs"
    >
      <StatusDot tone={status.tone} />
      {status.label}
    </Badge>
  );
}

function ActionButton({
  entry,
  size = 'sm',
}: {
  entry: EntryCardData;
  size?: 'sm' | 'xs';
}) {
  if (entry.running) {
    return (
      <ConfirmDialog
        trigger={
          <Button
            variant="outline"
            size={size}
            data-icon="inline-start"
            disabled={entry.busy}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
            }}
          >
            <PowerIcon weight="bold" />
            {m['action.stop']()}
          </Button>
        }
        title={m['entry.stop_title']({ name: entry.name })}
        description={
          entry.kind === 'server'
            ? m['entry.stop_server_description']()
            : m['entry.stop_instance_description']()
        }
        confirmLabel={m['action.stop']()}
        onConfirm={() => entry.onStop?.()}
      />
    );
  }
  return (
    <Button
      size={size}
      disabled={!entry.ready || entry.busy}
      data-icon="inline-start"
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        entry.onStart?.();
      }}
      className="bg-ember text-ember-foreground hover:bg-ember/90"
    >
      {entry.busy ? <Spinner /> : <PlayIcon weight="fill" />}
      {entry.kind === 'server' ? m['action.start']() : m['action.play']()}
    </Button>
  );
}

/** Grid tile: art banner + name + loader/version chips + footer. */
export function EntryCard({ entry }: { entry: EntryCardData }) {
  const status = statusOf(entry);
  const Icon = entryIcon(entry.kind);

  return (
    <Link
      to={detailTo(entry.kind)}
      params={{ id: entry.id }}
      className="group block outline-none focus-visible:ring-1 focus-visible:ring-ring"
    >
      <Card className="gap-0 overflow-hidden py-0 transition-colors group-hover:border-ember/40">
        <div className="relative flex h-24 items-center justify-center border-b border-border bg-muted/40">
          <Icon className="size-9 text-muted-foreground/40" />
          {status && (
            <div className="absolute top-2 left-2">
              <StatusBadge status={status} />
            </div>
          )}
          <div className="absolute right-2 bottom-2 opacity-0 transition-opacity group-hover:opacity-100">
            <ActionButton entry={entry} />
          </div>
        </div>

        <div className="space-y-2 p-3">
          <div className="truncate text-sm font-medium">{entry.name}</div>
          <div className="flex items-center gap-1.5">
            <Badge variant="secondary" className="uppercase">
              {entry.flavor}
            </Badge>
            <Badge variant="outline" className="font-mono">
              {entry.version}
            </Badge>
          </div>
          <div className="truncate font-mono text-[11px] text-muted-foreground">
            {entry.subtitle}
          </div>
        </div>
      </Card>
    </Link>
  );
}

/** List row: icon + name + chips inline + action. */
export function EntryRow({ entry }: { entry: EntryCardData }) {
  const status = statusOf(entry);
  const Icon = entryIcon(entry.kind);

  return (
    <Link
      to={detailTo(entry.kind)}
      params={{ id: entry.id }}
      className={cn(
        'flex items-center gap-3 px-3 py-2.5 outline-none transition-colors hover:bg-muted/40 focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset',
      )}
    >
      <span className="grid size-9 shrink-0 place-items-center bg-muted text-muted-foreground ring-1 ring-border">
        <Icon className="size-4.5" />
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-medium">{entry.name}</span>
          {status && <StatusBadge status={status} />}
        </div>
        <div className="truncate font-mono text-[11px] text-muted-foreground">
          {entry.flavor} · {entry.version} · {entry.subtitle}
        </div>
      </div>
      <ActionButton entry={entry} />
    </Link>
  );
}
