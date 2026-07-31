import { CaretDownIcon, PlayIcon, PowerIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import type { ProcessInfo } from '@/api';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { uptime } from '@/lib/format';
import { sessionSeq } from '@/lib/sessions';
import { m } from '@/paraglide/messages.js';

type Size = 'xs' | 'sm' | 'default';

const CARET_SIZE = {
  xs: 'icon-xs',
  sm: 'icon-sm',
  default: 'icon',
} as const;

/** `session` names one to stop; absent stops every session of the instance. */
type StopTarget = { session?: string };

/**
 * What a running instance offers: Stop, plus the menu a further launch and the
 * per-session stops hang off.
 *
 * Every handler stops propagation — the control renders inside the library's
 * card links, and the menu's portalled content still bubbles through the React
 * tree to them.
 */
export function InstanceRunControl({
  name,
  sessions,
  size = 'default',
  busy = false,
  launching = false,
  onNewSession,
  onStop,
}: {
  name: string;
  /** The live sessions, oldest first. */
  sessions: ProcessInfo[];
  size?: Size;
  busy?: boolean;
  launching?: boolean;
  onNewSession: () => void;
  onStop: (session?: string) => void;
}) {
  const [confirm, setConfirm] = useState<StopTarget | null>(null);
  const now = Date.now() / 1000;
  const target = confirm?.session;
  const seq = target ? sessionSeq(target) : 0;

  return (
    <>
      <div className="flex items-center">
        <Button
          variant="outline"
          size={size}
          data-icon="inline-start"
          disabled={busy}
          className="border-r-0"
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            setConfirm({});
          }}
        >
          <PowerIcon weight="bold" />
          {m['app.action.stop']()}
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button
                variant="outline"
                size={CARET_SIZE[size]}
                aria-label={m['entry.session.menu']()}
                title={m['entry.session.menu']()}
                disabled={busy}
                onClick={(event) => event.stopPropagation()}
              >
                <CaretDownIcon weight="bold" />
              </Button>
            }
          />
          <DropdownMenuContent
            align="end"
            className="w-56"
            onClick={(event) => event.stopPropagation()}
          >
            <DropdownMenuItem disabled={launching} onClick={onNewSession}>
              <PlayIcon weight="fill" />
              {m['entry.session.new']()}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={() => setConfirm({})}>
              {sessions.length > 1
                ? m['entry.session.stop_all']()
                : m['app.action.stop']()}
            </DropdownMenuItem>
            {sessions.length > 1 && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuLabel>
                  {m['app.label.sessions']()}
                </DropdownMenuLabel>
                {sessions.map((session) => (
                  <DropdownMenuItem
                    key={session.id}
                    onClick={() => setConfirm({ session: session.id })}
                  >
                    <span className="min-w-0 flex-1 truncate">
                      {m['entry.session.stop_one']({
                        seq: sessionSeq(session.id),
                      })}
                    </span>
                    <span className="font-mono text-[10px] text-muted-foreground">
                      {uptime(now - session.startedUnix)}
                    </span>
                  </DropdownMenuItem>
                ))}
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      <ConfirmDialog
        open={confirm != null}
        onOpenChange={(open) => {
          if (!open) setConfirm(null);
        }}
        title={
          target
            ? m['entry.stop.session_title']({ seq })
            : m['entry.stop.title']({ name })
        }
        description={
          target
            ? m['entry.stop.session_description']({
                count: sessions.length - 1,
              })
            : sessions.length > 1
              ? m['entry.stop.sessions_description']({
                  count: sessions.length,
                })
              : m['entry.stop.instance_description']()
        }
        confirmLabel={m['app.action.stop']()}
        onConfirm={() => {
          onStop(target);
          setConfirm(null);
        }}
      />
    </>
  );
}
