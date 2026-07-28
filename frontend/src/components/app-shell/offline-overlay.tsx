import { PlugsIcon } from '@phosphor-icons/react';

import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';
import { m } from '@/paraglide/messages.js';
import { useDaemon } from '@/queries/daemon';

/**
 * The one place a lost daemon is reported: reads stop answering app-wide, so
 * every page would otherwise render its own empty state. The shell reconnects
 * on its own, so this clears itself when the daemon comes back.
 */
export function OfflineOverlay() {
  const daemon = useDaemon();
  if (daemon.connected) return null;

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-background/80 backdrop-blur-xs">
      <div className="flex w-full max-w-md flex-col items-center gap-6 border border-border bg-card px-8 py-10 text-center shadow-lg">
        <PlugsIcon className="size-10 text-muted-foreground" weight="duotone" />
        <div className="space-y-2">
          <h2 className="text-xl font-semibold">
            {m['app.daemon.offline_title']()}
          </h2>
          <p className="text-sm text-muted-foreground">
            {m['app.daemon.offline_body']()}
          </p>
        </div>
        <Button
          className="w-full"
          disabled={daemon.busy}
          onClick={() => daemon.start.mutate()}
          data-icon="inline-start"
        >
          {daemon.busy && <Spinner className="size-4" />}
          {daemon.busy ? m['app.daemon.starting']() : m['app.daemon.start']()}
        </Button>
      </div>
    </div>
  );
}
