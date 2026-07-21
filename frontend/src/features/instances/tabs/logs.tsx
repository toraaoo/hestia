import { Empty } from '@/components/empty';
import { LogView } from '@/components/log-view';
import { m } from '@/paraglide/messages.js';
import { useInstanceLogs } from '@/queries/instance';

/** The newest running session's captured output, followed while it runs. */
export function InstanceLogsTab({
  id,
  running,
  name,
}: {
  id: string;
  running: boolean;
  name: string;
}) {
  const logs = useInstanceLogs(id, { follow: running, tail: 500 });

  if (logs.lines.length === 0) {
    return <Empty className="h-full">{m['detail.logs_empty']()}</Empty>;
  }

  return (
    <LogView
      aria-label={name}
      rows={logs.lines.map((entry) => ({ text: entry.line }))}
    />
  );
}
