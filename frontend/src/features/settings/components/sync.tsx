import { useMutation, useQuery } from '@tanstack/react-query';

import type { SyncTargets } from '@/api';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
import { Switch } from '@/components/ui/switch';
import {
  AddRow,
  Setting,
  SwitchRow,
  ValueRow,
} from '@/features/settings/components';
import { m } from '@/paraglide/messages.js';
import { syncMutations, syncQueries } from '@/queries/sync';

interface Known {
  name: string;
  description: () => string;
}

/**
 * What Minecraft writes into an instance — offered rather than remembered. The
 * launcher-managed content directories are deliberately absent: the daemon
 * refuses them as targets, so offering one is offering an error.
 */
const KNOWN_FILES: Known[] = [
  { name: 'options.txt', description: m['settings.sync.target.options_txt'] },
  { name: 'servers.dat', description: m['settings.sync.target.servers_dat'] },
];

const KNOWN_FOLDERS: Known[] = [
  { name: 'saves', description: m['settings.sync.target.saves'] },
  { name: 'config', description: m['settings.sync.target.config'] },
  { name: 'screenshots', description: m['settings.sync.target.screenshots'] },
];

/**
 * The launcher-wide sync settings: whether instances share at all and which
 * targets they share. Where one instance stands — and whether it takes part —
 * belongs to that instance's own settings, not here.
 */
export function SyncSettings({
  onCommit,
}: {
  onCommit: (key: string, value: unknown) => void;
}) {
  const config = useQuery(syncQueries.config());
  const setTargets = useMutation(syncMutations.set());

  const targets = config.data?.targets ?? { files: [], folders: [] };
  const enabled = config.data?.enabled ?? true;

  const change = (next: SyncTargets) => setTargets.mutate(next);

  return (
    <>
      <Setting id="sync-enabled">
        <SwitchRow
          id="sync-enabled"
          label={m['settings.sync.enabled_label']()}
          description={m['settings.sync.enabled_description']()}
          checked={enabled}
          disabled={config.isPending}
          onChange={(checked) => onCommit('sync.enabled', checked)}
        />
      </Setting>

      {enabled && (
        <Setting id="sync-targets">
          {config.isPending ? (
            <div className="flex flex-col gap-2">
              <Bone className="h-28" />
              <Bone className="h-28" />
            </div>
          ) : (
            <div className="flex flex-col gap-5">
              <TargetGroup
                label={m['settings.sync.files']()}
                hint={m['settings.sync.files_hint']()}
                placeholder={m['settings.sync.add_file_placeholder']()}
                known={KNOWN_FILES}
                values={targets.files}
                pending={setTargets.isPending}
                onChange={(files) => change({ ...targets, files })}
              />
              <TargetGroup
                label={m['settings.sync.folders']()}
                hint={m['settings.sync.folders_hint']()}
                placeholder={m['settings.sync.add_folder_placeholder']()}
                known={KNOWN_FOLDERS}
                values={targets.folders}
                pending={setTargets.isPending}
                onChange={(folders) => change({ ...targets, folders })}
              />
            </div>
          )}
        </Setting>
      )}
    </>
  );
}

/**
 * One side of the target set: the targets Minecraft is known to write, each a
 * switch, and whatever else the set holds as rows under them.
 */
function TargetGroup({
  label,
  hint,
  placeholder,
  known,
  values,
  pending,
  onChange,
}: {
  label: string;
  hint: string;
  placeholder: string;
  known: Known[];
  values: string[];
  pending: boolean;
  onChange: (values: string[]) => void;
}) {
  const names = new Set(known.map((entry) => entry.name));
  const custom = values.filter((value) => !names.has(value));

  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <FieldDescription>{hint}</FieldDescription>
      <div className="divide-y divide-border border border-border">
        {known.map((entry) => (
          <label
            key={entry.name}
            htmlFor={`sync-${entry.name}`}
            className="flex cursor-pointer items-center gap-3 px-3 py-2"
          >
            <div className="min-w-0 flex-1">
              <div className="font-mono text-xs">{entry.name}</div>
              <div className="text-[11px] text-muted-foreground">
                {entry.description()}
              </div>
            </div>
            <Switch
              id={`sync-${entry.name}`}
              size="sm"
              checked={values.includes(entry.name)}
              disabled={pending}
              onCheckedChange={(checked) =>
                onChange(
                  checked === true
                    ? [...values, entry.name]
                    : values.filter((v) => v !== entry.name),
                )
              }
            />
          </label>
        ))}

        {custom.map((value) => (
          <ValueRow
            key={value}
            value={value}
            badge={
              <Badge variant="outline">{m['settings.sync.custom']()}</Badge>
            }
            pending={pending}
            onRemove={() => onChange(values.filter((v) => v !== value))}
          />
        ))}

        <AddRow
          placeholder={placeholder}
          label={m['settings.sync.add_custom']()}
          pending={pending}
          onAdd={(value) => {
            if (!values.includes(value)) onChange([...values, value]);
          }}
        />
      </div>
    </Field>
  );
}
