import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';

import { Empty } from '@/components/empty';
import { type LogRow, LogView } from '@/components/log-view';
import { Input } from '@/components/ui/input';
import { m } from '@/paraglide/messages.js';
import {
  type ConsoleEntry,
  pushConsoleEntry,
  useConsoleHistory,
} from '@/queries/console';
import { serverMutations, useServerLogs } from '@/queries/server';

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
  const command = useMutation(serverMutations.command(id));
  const [line, setLine] = useState('');
  const entries = useConsoleHistory(id);

  if (!running) {
    return <Empty className="h-full">{m['detail.console_empty']()}</Empty>;
  }

  const push = (entry: ConsoleEntry) => pushConsoleEntry(id, entry);

  // Captured output first, then this session's command echoes and RCON replies.
  const rows: LogRow[] = [
    ...logs.lines.map((entry) => ({ text: entry.line })),
    ...entries.map((entry) => ({
      text: entry.text,
      className:
        entry.kind === 'echo'
          ? 'text-foreground/70'
          : entry.kind === 'error'
            ? 'text-destructive'
            : undefined,
    })),
  ];

  const send = () => {
    const text = line.trim();
    if (!text) return;
    setLine('');
    push({ kind: 'echo', text: `» ${text}` });
    command.mutate(text, {
      onSuccess: (reply) => {
        const trimmed = reply.trim();
        if (trimmed) push({ kind: 'reply', text: trimmed });
      },
      onError: (error) => push({ kind: 'error', text: error.message }),
    });
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      <LogView rows={rows} emptyLabel={`${name} — ${m['status.online']()}`} />
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
