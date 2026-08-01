import { PlayIcon, PowerIcon, PushPinIcon } from '@phosphor-icons/react';
import { createLink } from '@tanstack/react-router';
import { motion } from 'motion/react';

import type { ProcessInfo } from '@/api';
import { entryIcon } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { Spinner } from '@/components/ui/spinner';
import { StatusDot } from '@/components/ui/status-dot';
import { EntryRunControl } from '@/features/shared/entry/components';
import { layoutMorph, listItem } from '@/lib/motion';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { usePinned } from '@/queries/pinned';
import type { View } from './collection';

export interface EntryCardModel {
  id: string;
  name: string;
  kind: 'instance' | 'server';
  flavor: string;
  version: string;
  running: boolean;
  ready: boolean;
  /** One-line footer: "Last played 2h ago" / ":25565 · 3 online". */
  subtitle: string;
  /** A custom icon shown in place of the kind glyph. */
  iconUrl?: string;
  /** An instance's live sessions, oldest first; a server has none. */
  sessions?: ProcessInfo[];
  /** Wired quick actions; absent leaves the button inert. */
  onStart?: () => void;
  /** `session` names one to stop; absent stops the entry outright. */
  onStop?: (session?: string) => void;
  /** Launch alongside the running sessions; absent hides the option. */
  onNewSession?: () => void;
  busy?: boolean;
  /** Split out of `busy` so a launch in flight still leaves stopping live. */
  launching?: boolean;
  stopping?: boolean;
}

const MotionLink = createLink(motion.a);

function statusOf(entry: EntryCardModel) {
  if (!entry.ready)
    return { tone: 'warn' as const, label: m['app.status.preparing']() };
  if (entry.running)
    return {
      tone: 'on' as const,
      label:
        entry.kind === 'server'
          ? m['app.status.online']()
          : m['app.status.running'](),
    };
  return null;
}

function detailTo(kind: 'instance' | 'server') {
  return kind === 'server' ? '/servers/$id' : '/instances/$id';
}

function StatusBadge({
  status,
  overlay,
}: {
  status: NonNullable<ReturnType<typeof statusOf>>;
  overlay?: boolean;
}) {
  return (
    <Badge
      variant="secondary"
      className={cn('gap-1.5', overlay && 'bg-background/80 backdrop-blur-xs')}
    >
      <StatusDot tone={status.tone} />
      {status.label}
    </Badge>
  );
}

/** Sidebar pin toggle: visible on hover, or always while pinned. */
function PinToggle({
  entry,
  overlay,
}: {
  entry: EntryCardModel;
  overlay?: boolean;
}) {
  const { ready, isPinned, toggle } = usePinned();
  const pin = { kind: entry.kind, id: entry.id };
  const pinned = isPinned(pin);
  if (!ready) return null;

  const label = pinned ? m['app.label.unpin']() : m['app.label.pin']();
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      aria-pressed={pinned}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        toggle(pin);
      }}
      className={cn(
        'grid size-6 place-items-center text-muted-foreground opacity-0 transition-opacity outline-none group-hover:opacity-100 hover:text-foreground focus-visible:opacity-100 focus-visible:ring-ring',
        overlay && 'bg-background/80 ring-1 ring-border backdrop-blur-xs',
      )}
    >
      <PushPinIcon weight={pinned ? 'fill' : 'regular'} className="size-3.5" />
    </button>
  );
}

function ActionButton({
  entry,
  size = 'sm',
}: {
  entry: EntryCardModel;
  size?: 'sm' | 'xs';
}) {
  if (entry.running && entry.onNewSession) {
    return (
      <EntryRunControl
        name={entry.name}
        sessions={entry.sessions ?? []}
        size={size}
        busy={entry.stopping}
        launching={entry.launching}
        onNewSession={entry.onNewSession}
        onStop={(session) => entry.onStop?.(session)}
      />
    );
  }
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
            {m['app.action.stop']()}
          </Button>
        }
        title={m['entry.stop.title']({ name: entry.name })}
        description={
          entry.kind === 'server'
            ? m['entry.stop.server_description']()
            : m['entry.stop.instance_description']()
        }
        confirmLabel={m['app.action.stop']()}
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
      {entry.kind === 'server'
        ? m['app.action.start']()
        : m['app.action.play']()}
    </Button>
  );
}

export function EntryTile({
  entry,
  view,
}: {
  entry: EntryCardModel;
  view: View;
}) {
  const status = statusOf(entry);
  const Icon = entryIcon(entry.kind);
  const grid = view === 'grid';

  return (
    <MotionLink
      layout
      variants={listItem}
      exit="exit"
      transition={layoutMorph}
      to={detailTo(entry.kind)}
      params={{ id: entry.id }}
      className={cn(
        'group relative flex outline-none focus-visible:ring-1 focus-visible:ring-ring',
        grid
          ? 'flex-col overflow-hidden border border-border bg-card transition-colors hover:border-ember/40'
          : 'items-center gap-3 px-3 py-2.5 transition-colors hover:bg-muted/40 focus-visible:ring-inset',
      )}
    >
      <motion.div
        layout
        transition={layoutMorph}
        className={cn(
          'relative grid shrink-0 place-items-center overflow-hidden',
          grid
            ? 'h-24 w-full border-b border-border bg-muted/40'
            : 'size-9 bg-muted ring-1 ring-border',
        )}
      >
        {entry.iconUrl ? (
          <img src={entry.iconUrl} alt="" className="size-full object-cover" />
        ) : (
          <Icon
            className={cn(
              'text-muted-foreground',
              grid ? 'size-9 opacity-40' : 'size-4.5',
            )}
          />
        )}
      </motion.div>

      <motion.div
        layout
        transition={layoutMorph}
        className={cn('min-w-0', grid ? 'w-full space-y-2 p-3' : 'flex-1')}
      >
        <span className="block truncate text-sm font-medium">{entry.name}</span>
        {grid && (
          <div className="flex items-center gap-1.5">
            <Badge variant="secondary" className="uppercase">
              {entry.flavor}
            </Badge>
            <Badge variant="outline" className="font-mono">
              {entry.version}
            </Badge>
          </div>
        )}
        <span className="block truncate font-mono text-[11px] text-muted-foreground">
          {grid
            ? entry.subtitle
            : `${entry.flavor} · ${entry.version} · ${entry.subtitle}`}
        </span>
      </motion.div>

      {status && (
        <motion.div
          layout
          transition={layoutMorph}
          className={cn(grid && 'absolute top-2 left-2')}
        >
          <StatusBadge status={status} overlay={grid} />
        </motion.div>
      )}

      <motion.div
        layout
        transition={layoutMorph}
        className={cn(grid && 'absolute top-2 right-2')}
      >
        <PinToggle entry={entry} overlay={grid} />
      </motion.div>

      <motion.div
        layout
        transition={layoutMorph}
        className={cn(
          grid &&
            'absolute top-15 right-2 opacity-0 transition-opacity duration-150 group-hover:opacity-100 has-aria-expanded:opacity-100',
        )}
      >
        <ActionButton entry={entry} />
      </motion.div>
    </MotionLink>
  );
}
