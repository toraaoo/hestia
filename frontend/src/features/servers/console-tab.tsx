import { useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';

import { Empty } from '@/components/empty';
import { Input } from '@/components/ui/input';
import { m } from '@/paraglide/messages.js';
import { useServerCommand, useServerLogs } from '@/queries/server';

export function ServerConsoleTab({
  id,
  running,
  name,
}: {
  id: string;
  running: boolean;
  name: string;
}) {
  const logs = useServerLogs(id, { follow: running, tail: 500 });
  const command = useServerCommand(id);
  const [line, setLine] = useState('');
  const scrollRef = useRef<HTMLDivElement>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: pin to the tail on new output.
  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [logs.lines.length]);

  if (!running) {
    return <Empty>{m['detail.console_empty']()}</Empty>;
  }

  const send = () => {
    const text = line.trim();
    if (!text) return;
    setLine('');
    command.mutate(text, {
      onSuccess: (reply) => {
        if (reply.trim()) toast.message(reply.trim());
      },
      onError: (error) => toast.error(error.message),
    });
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      <div
        ref={scrollRef}
        className="min-h-0 flex-1 space-y-0.5 overflow-y-auto border border-border bg-card p-3 font-mono text-[11px] text-muted-foreground"
      >
        {logs.lines.length === 0 ? (
          <span className="text-muted-foreground/60">
            {name} — {m['status.online']()}
          </span>
        ) : (
          logs.lines.map((entry, index) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: log lines have no stable id.
            <div key={index}>{entry.line}</div>
          ))
        )}
      </div>
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          send();
        }}
      >
        <Input
          placeholder={m['detail.console_placeholder']()}
          className="font-mono"
          value={line}
          onChange={(e) => setLine(e.target.value)}
          disabled={command.isPending}
        />
      </form>
    </div>
  );
}
