import { Empty } from '@/components/empty';
import { LogView } from '@/components/log-view';
import { m } from '@/paraglide/messages.js';
import { useInstanceLogs } from '@/queries/instance';

/**
 * The instance's captured output, followed continuously: the subject is the
 * instance, so a session ending leaves the log in place and the next launch
 * streams into it.
 */
export function InstanceLogsTab({ id, name }: { id: string; name: string }) {
  const logs = useInstanceLogs(id, { follow: true, tail: 500 });

  if (logs.lines.length === 0) {
    return <Empty className="h-full">{m['instance.logs_empty']()}</Empty>;
  }

  return (
    <LogView
      aria-label={name}
      rows={logs.lines.map((entry) => ({ text: entry.line }))}
    />
  );
}
