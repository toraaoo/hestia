import { useMutation, useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';

import type { LinkState } from '@/api';
import { Bone } from '@/components/skeleton';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
import { StatusDot } from '@/components/ui/status-dot';
import { Switch } from '@/components/ui/switch';
import { cn } from '@/lib/utils';
import { toastWarnings } from '@/lib/warnings';
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

/**
 * Whether this instance shares its settings with the others, and where each of
 * its folder targets stands when it does. Taking it out or bringing it back
 * moves files either way, so both directions confirm first and report what it
 * cost.
 */
export function InstanceSyncField({
  id,
  name,
  running,
}: {
  id: string;
  name: string;
  running: boolean;
}) {
  const config = useQuery(syncQueries.config());
  const status = useQuery(syncQueries.status());
  const share = useMutation(syncMutations.share(id));
  const [pending, setPending] = useState<boolean | null>(null);

  const mine = status.data?.find((instance) => instance.id === id);
  const sharing = mine?.enabled ?? true;

  const apply = () => {
    if (pending === null) return;
    share.mutate(pending, {
      onSuccess: (result) => {
        toastWarnings(result.warnings);
        toast.success(
          result.enabled
            ? m['instance.sync.now_sharing']()
            : m['instance.sync.now_alone'](),
        );
      },
      onSettled: () => setPending(null),
    });
  };

  if (config.isPending || status.isPending) return <Bone className="h-16" />;

  return (
    <Field>
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <FieldLabel htmlFor="instance-sync">
            {m['instance.sync.title']()}
          </FieldLabel>
          <FieldDescription>
            {config.data?.enabled === false
              ? m['instance.sync.off_launcher_wide']()
              : sharing
                ? m['instance.sync.description']()
                : m['instance.sync.opted_out']()}
          </FieldDescription>
        </div>
        <ConfirmDialog
          open={pending !== null}
          onOpenChange={(open) => !open && setPending(null)}
          trigger={
            <Switch
              id="instance-sync"
              aria-label={m['instance.sync.share_label']()}
              checked={sharing}
              disabled={
                running || share.isPending || config.data?.enabled === false
              }
              onCheckedChange={(checked) => setPending(checked === true)}
            />
          }
          title={
            pending
              ? m['instance.sync.join_title']({ name })
              : m['instance.sync.leave_title']({ name })
          }
          description={
            pending
              ? m['instance.sync.join_description']()
              : m['instance.sync.leave_description']()
          }
          confirmLabel={
            pending
              ? m['instance.sync.join_action']()
              : m['instance.sync.leave_action']()
          }
          destructive={pending === true}
          onConfirm={apply}
        />
      </div>

      {sharing && config.data?.enabled !== false && (
        <TargetStates id={id} name={name} targets={mine?.targets ?? []} />
      )}
    </Field>
  );
}

function TargetStates({
  id,
  name,
  targets,
}: {
  id: string;
  name: string;
  targets: { target: string; state: LinkState }[];
}) {
  if (targets.length === 0) {
    return (
      <FieldDescription>{m['instance.sync.no_targets']()}</FieldDescription>
    );
  }
  return (
    <div className="divide-y divide-border border border-border">
      {targets.map((target) => (
        <div
          key={target.target}
          className="flex items-center gap-3 px-3 py-1.5 text-xs"
        >
          <StatusDot tone={stateTone[target.state]} />
          <span className="min-w-0 flex-1 truncate font-mono">
            {target.target}
          </span>
          <span
            className={cn(
              'text-muted-foreground',
              target.state === 'cannot_link' && 'text-destructive',
            )}
          >
            {stateLabel[target.state]()}
          </span>
          {target.state === 'cannot_link' && (
            <AdoptButton id={id} name={name} target={target.target} />
          )}
        </div>
      ))}
    </div>
  );
}

/** Moving one clashing folder's contents into the store, once the user says so. */
function AdoptButton({
  id,
  name,
  target,
}: {
  id: string;
  name: string;
  target: string;
}) {
  const adopt = useMutation(syncMutations.adopt(id));
  return (
    <ConfirmDialog
      trigger={
        <Button variant="outline" size="xs" disabled={adopt.isPending}>
          {m['instance.sync.adopt.action']()}
        </Button>
      }
      title={`${m['instance.sync.adopt.action']()} — ${name}`}
      description={m['instance.sync.adopt.description']({ target })}
      confirmLabel={m['instance.sync.adopt.action']()}
      onConfirm={() =>
        adopt.mutate([target], {
          onSuccess: (adopted) =>
            toast.success(
              m['instance.sync.adopt.done']({ targets: adopted.join(', ') }),
            ),
        })
      }
    />
  );
}
