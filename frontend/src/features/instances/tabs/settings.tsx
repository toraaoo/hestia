import { useMutation, useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';

import type { ConfigEntry, InstanceInfo } from '@/api';
import { EntrySettingsTab } from '@/features/entries/settings';
import { m } from '@/paraglide/messages.js';
import { configQueries, launcherDefaults } from '@/queries/config';
import { instanceMutations, instanceQueries } from '@/queries/instance';

export function InstanceSettingsTab({
  instance,
  config,
  running,
}: {
  instance: InstanceInfo;
  config?: ConfigEntry[];
  running: boolean;
}) {
  const globalConfig = useQuery(configQueries.list());
  const rename = useMutation(instanceMutations.rename(instance.id));
  const setConfig = useMutation(instanceMutations.setConfig(instance.id));
  const remove = useMutation(instanceMutations.remove(instance.id));
  const update = useMutation(instanceMutations.update(instance.id));

  return (
    <EntrySettingsTab
      kind="instance"
      entry={instance}
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
      versionsQuery={instanceQueries.versions(instance.flavor)}
    />
  );
}
