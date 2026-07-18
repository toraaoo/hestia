import { DownloadSimpleIcon } from '@phosphor-icons/react';

import { Progress } from '@/components/ui/progress';
import { StatusDot } from '@/components/ui/status-dot';
import { m } from '@/paraglide/messages.js';
import { useDaemon } from '@/queries/daemon';

export function StatusBar() {
  const daemon = useDaemon();
  const jobProgress = 64;

  return (
    <footer className="flex h-8 shrink-0 items-center gap-3 border-t border-border bg-sidebar px-4 text-[11px] text-muted-foreground">
      <span className="inline-flex items-center gap-1.5">
        <StatusDot tone={daemon.connected ? 'on' : 'off'} />
        {daemon.connected ? m['daemon.connected']() : m['daemon.offline']()}
      </span>
      {daemon.status && (
        <span className="font-mono">v{daemon.status.version}</span>
      )}

      <div className="ml-auto flex items-center gap-2">
        <DownloadSimpleIcon className="size-3.5" />
        <span>{m['job.installing']({ name: 'Sodium' })}</span>
        <Progress value={jobProgress} className="w-28" />
        <span className="font-mono tabular-nums">{jobProgress}%</span>
      </div>
    </footer>
  );
}
