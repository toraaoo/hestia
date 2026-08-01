import { useMutation, useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';

import type { LinkState, SyncTargets } from '@/api';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
import { StatusDot } from '@/components/ui/status-dot';
import { Switch } from '@/components/ui/switch';
import {
  AddRow,
  Setting,
  SwitchRow,
  ValueRow,
} from '@/features/settings/components';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { syncMutations, syncQueries } from '@/queries/sync';

const stateLabel: Record<LinkState, () => string> = {
  linked: () => m['domain.sync_state.linked'](),
  pending: () => m['domain.sync_state.pending'](),
  cannot_link: () => m['domain.sync_state.cannot_link'](),
};

const stateTone: Record<LinkState, 'on' | 'off' | 'warn'> = {
  linked: 'on',
  pending: 'off',
  cannot_link: 'warn',
};

interface Known {
  name: string;
  description: () => string;
}

/** What Minecraft writes into an instance — offered rather than remembered. */
const KNOWN_FILES: Known[] = [
  { name: 'options.txt', description: m['settings.sync.target.options_txt'] },
  { name: 'servers.dat', description: m['settings.sync.target.servers_dat'] },
];

const KNOWN_FOLDERS: Known[] = [
  { name: 'saves', description: m['settings.sync.target.saves'] },
  { name: 'config', description: m['settings.sync.target.config'] },
  { name: 'screenshots', description: m['settings.sync.target.screenshots'] },
  {
    name: 'resourcepacks',
    description: m['settings.sync.target.resourcepacks'],
  },
  { name: 'shaderpacks', description: m['settings.sync.target.shaderpacks'] },
];

/**
 * The instance-sync settings: whether instances share at all, the shared target
 * set (files copied, folders linked), and every instance's per-folder link
 * state — with adopt for a folder whose names clash with the store.
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

      {enabled && (
        <Setting id="sync-status">
          <LinkStatus hasFolderTargets={targets.folders.length > 0} />
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

/** Where each instance stands against the store, and what to do about it. */
function LinkStatus({ hasFolderTargets }: { hasFolderTargets: boolean }) {
  const status = useQuery(syncQueries.status());

  return (
    <Field>
      <FieldLabel>{m['settings.sync.status_title']()}</FieldLabel>
      {status.isPending ? (
        <Bone className="h-10" />
      ) : !hasFolderTargets ? (
        <FieldDescription>
          {m['settings.sync.no_folder_targets']()}
        </FieldDescription>
      ) : (
        <div className="divide-y divide-border border border-border">
          {(status.data ?? []).map((instance) => (
            <div
              key={instance.id}
              className="flex flex-wrap items-center gap-x-4 gap-y-1 px-3 py-2"
            >
              <span className="min-w-0 flex-1 truncate text-sm">
                {instance.name}
              </span>
              <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
                {instance.targets.map((target) => (
                  <span
                    key={target.target}
                    className="flex items-center gap-1.5 text-xs"
                  >
                    <StatusDot tone={stateTone[target.state]} />
                    <span className="font-mono">{target.target}</span>
                    {target.state !== 'linked' && (
                      <span
                        className={cn(
                          'text-muted-foreground',
                          target.state === 'cannot_link' && 'text-destructive',
                        )}
                      >
                        {stateLabel[target.state]()}
                      </span>
                    )}
                  </span>
                ))}
              </div>
              {instance.targets.some((t) => t.state === 'cannot_link') && (
                <AdoptButton id={instance.id} name={instance.name} />
              )}
            </div>
          ))}
        </div>
      )}
    </Field>
  );
}

function AdoptButton({ id, name }: { id: string; name: string }) {
  const adopt = useMutation(syncMutations.adopt(id));
  return (
    <ConfirmDialog
      trigger={
        <Button variant="outline" size="xs" disabled={adopt.isPending}>
          {m['settings.sync.adopt.action']()}
        </Button>
      }
      title={`${m['settings.sync.adopt.action']()} — ${name}`}
      description={m['settings.sync.adopt.description']()}
      confirmLabel={m['settings.sync.adopt.action']()}
      onConfirm={() =>
        adopt.mutate(undefined, {
          onSuccess: (adopted) =>
            toast.success(
              m['settings.sync.adopt.done']({ targets: adopted.join(', ') }),
            ),
        })
      }
    />
  );
}
