import { useMutation, useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';

import type { ConfigEntry, ServerInfo } from '@/api';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useDraft } from '@/features/shared/entry/hooks';
import {
  configValue,
  EntrySettingsTab,
} from '@/features/shared/entry/settings';
import { m } from '@/paraglide/messages.js';
import { configQueries, launcherDefaults } from '@/queries/config';
import { useJobMutation } from '@/queries/jobs';
import { serverMutations, serverQueries } from '@/queries/server';

const INTERVALS = ['off', '6h', '12h', '1d'];

/** Scheduled backups — the one setting an instance has no equivalent for. */
function useBackupSchedule(config?: ConfigEntry[]) {
  const loaded = config !== undefined;
  const storedInterval = configValue(config, 'backup-interval') || 'off';
  const storedRetention = configValue(config, 'backup-retention') || '10';

  const [interval, setInterval] = useDraft(
    storedInterval,
    `${loaded}:${storedInterval}`,
  );
  const [retention, setRetention] = useDraft(
    storedRetention,
    `${loaded}:${storedRetention}`,
  );

  const fields = (
    <div className="grid gap-4 sm:grid-cols-2">
      <Field>
        <FieldLabel htmlFor="backup-interval">
          {m['entry.settings.backup_schedule']()}
        </FieldLabel>
        <Select value={interval} onValueChange={(v) => setInterval(v ?? 'off')}>
          <SelectTrigger id="backup-interval" className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {INTERVALS.map((iv) => (
              <SelectItem key={iv} value={iv}>
                {iv === 'off'
                  ? m['app.label.off']()
                  : m['entry.settings.every_interval']({ interval: iv })}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <Field>
        <FieldLabel htmlFor="backup-retention">
          {m['entry.settings.keep_backups']()}
        </FieldLabel>
        <Input
          id="backup-retention"
          type="number"
          min={1}
          value={retention}
          onChange={(e) => setRetention(e.target.value)}
        />
      </Field>
    </div>
  );

  const entries: ConfigEntry[] = [
    { key: 'backup-interval', value: interval === 'off' ? '' : interval },
    { key: 'backup-retention', value: retention },
  ];

  return { fields, entries };
}

export function ServerSettingsTab({
  server,
  config,
  running,
}: {
  server: ServerInfo;
  config?: ConfigEntry[];
  running: boolean;
}) {
  const globalConfig = useQuery(configQueries.list());
  const rename = useMutation(serverMutations.rename(server.id));
  const setConfig = useMutation(serverMutations.setConfig(server.id));
  const remove = useMutation(serverMutations.remove(server.id));
  const update = useJobMutation(serverMutations.update(server.id));
  const backups = useBackupSchedule(config);

  return (
    <EntrySettingsTab
      kind="server"
      entry={server}
      config={config}
      defaults={launcherDefaults(globalConfig.data)}
      running={running}
      rename={{
        mutate: (name) =>
          rename.mutate(name, {
            onSuccess: (updated) =>
              toast.success(m['app.toast.renamed']({ name: updated.name })),
          }),
        isPending: rename.isPending,
      }}
      setConfig={setConfig}
      remove={remove}
      update={update}
      versionsQuery={serverQueries.versions(server.flavor)}
      extraFields={backups.fields}
      extraConfig={backups.entries}
    />
  );
}
