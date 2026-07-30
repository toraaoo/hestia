import { useMutation, useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';

import type { LinkState, SyncTargets } from '@/api';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from '@/components/ui/field';
import { CheckboxRow, TargetList } from '@/features/settings/components/fields';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { configMutations } from '@/queries/config';
import { syncMutations, syncQueries } from '@/queries/sync';

const stateLabel: Record<LinkState, () => string> = {
  linked: () => m['domain.sync_state.linked'](),
  pending: () => m['domain.sync_state.pending'](),
  cannot_link: () => m['domain.sync_state.cannot_link'](),
};

/**
 * The instance-sync settings: whether instances share at all, the shared target
 * set (files copied, folders linked), and every instance's per-folder link
 * state — with adopt for a folder whose names clash with the store.
 */
export function SyncSection() {
  const config = useQuery(syncQueries.config());
  const status = useQuery(syncQueries.status());
  const setTargets = useMutation(syncMutations.set());
  const setConfig = useMutation(configMutations.set());

  const targets = config.data?.targets ?? { files: [], folders: [] };
  const enabled = config.data?.enabled ?? true;

  const commit = (next: SyncTargets) => setTargets.mutate(next);

  return (
    <FieldSet>
      <FieldLegend>{m['settings.sync.section']()}</FieldLegend>
      <FieldGroup>
        <FieldDescription>{m['settings.sync.description']()}</FieldDescription>

        <Field>
          <CheckboxRow
            id="sync-enabled"
            label={m['settings.sync.enabled_label']()}
            checked={enabled}
            disabled={config.isPending || setConfig.isPending}
            onChange={(checked) =>
              setConfig.mutate({ key: 'sync.enabled', value: checked })
            }
          />
          <FieldDescription>
            {m['settings.sync.enabled_description']()}
          </FieldDescription>
        </Field>

        {!enabled ? null : config.isPending ? (
          <div className="space-y-2">
            <Bone className="h-9" />
            <Bone className="h-9" />
          </div>
        ) : (
          <>
            <TargetList
              label={m['settings.sync.files']()}
              placeholder={m['settings.sync.add_file_placeholder']()}
              values={targets.files}
              pending={setTargets.isPending}
              onChange={(files) => commit({ ...targets, files })}
            />
            <TargetList
              label={m['settings.sync.folders']()}
              placeholder={m['settings.sync.add_folder_placeholder']()}
              values={targets.folders}
              pending={setTargets.isPending}
              onChange={(folders) => commit({ ...targets, folders })}
            />
          </>
        )}

        {enabled && (
          <Field>
            <FieldLabel>{m['settings.sync.status_title']()}</FieldLabel>
            {status.isPending ? (
              <Bone className="h-10" />
            ) : targets.folders.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                {m['settings.sync.no_folder_targets']()}
              </p>
            ) : (
              <div className="divide-y divide-border border border-border">
                {(status.data ?? []).map((inst) => (
                  <div
                    key={inst.id}
                    className="flex flex-wrap items-center gap-2 px-3 py-2"
                  >
                    <span className="min-w-0 flex-1 truncate text-sm">
                      {inst.name}
                    </span>
                    {inst.targets.map((t) => (
                      <Badge
                        key={t.target}
                        variant={t.state === 'linked' ? 'secondary' : 'outline'}
                        className={cn(
                          'font-mono text-[10px]',
                          t.state === 'cannot_link' && 'text-destructive',
                        )}
                      >
                        {t.target}: {stateLabel[t.state]()}
                      </Badge>
                    ))}
                    {inst.targets.some((t) => t.state === 'cannot_link') && (
                      <AdoptButton id={inst.id} name={inst.name} />
                    )}
                  </div>
                ))}
              </div>
            )}
          </Field>
        )}
      </FieldGroup>
    </FieldSet>
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
