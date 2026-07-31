import { SwitchRow } from '@/features/settings/components/controls';
import {
  Setting,
  SettingsSection,
} from '@/features/settings/components/filtered';
import { SyncSettings } from '@/features/settings/components/sync';
import { useConfig } from '@/features/settings/use-config';
import { m } from '@/paraglide/messages.js';

export function InstancesTab() {
  const { entries, commit } = useConfig();

  return (
    <>
      <SettingsSection
        group="instance-behaviour"
        legend={m['settings.instances.section']()}
      >
        <Setting id="multi-session">
          <SwitchRow
            id="multi-session"
            label={m['settings.instances.multi_session_label']()}
            description={m['settings.instances.multi_session_description']()}
            checked={entries.instance?.['multi-session'] ?? false}
            onChange={(checked) => commit('instance.multi-session', checked)}
          />
        </Setting>
      </SettingsSection>

      <SettingsSection
        group="sync"
        legend={m['settings.sync.section']()}
        description={m['settings.sync.description']()}
      >
        <SyncSettings onCommit={commit} />
      </SettingsSection>
    </>
  );
}
