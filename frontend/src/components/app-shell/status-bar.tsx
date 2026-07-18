import { StatusDot } from '@/components/ui/status-dot';
import { m } from '@/paraglide/messages.js';
import { useDaemon } from '@/queries/daemon';

export function StatusBar() {
  const daemon = useDaemon();

  return (
    <footer className="flex h-8 shrink-0 items-center gap-3 border-t border-border bg-sidebar px-4 text-[11px] text-muted-foreground">
      <span className="inline-flex items-center gap-1.5">
        <StatusDot tone={daemon.connected ? 'on' : 'off'} />
        {daemon.connected ? m['daemon.connected']() : m['daemon.offline']()}
      </span>
      {daemon.status && (
        <span className="font-mono">v{daemon.status.version}</span>
      )}
    </footer>
  );
}
