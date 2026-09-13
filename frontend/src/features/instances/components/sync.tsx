import { revalidateLogic } from '@tanstack/react-form';
import { useMutation, useQuery } from '@tanstack/react-query';
import { z } from 'zod';

import { useAppForm } from '@/components/form';
import { Bone } from '@/components/skeleton';
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
import { StatusDot } from '@/components/ui/status-dot';
import { Switch } from '@/components/ui/switch';
import { ValueRow } from '@/features/settings/components';
import { SYNC_UNITS, stateLabel, stateTone, unitLabel } from '@/lib/sync';
import { m } from '@/paraglide/messages.js';
import { syncMutations, syncQueries } from '@/queries/sync';

/**
 * What this instance takes from the shared settings, and which game settings it
 * keeps to itself. Nothing here moves files: the next launch applies it.
 */
export function InstanceSyncField({ id }: { id: string }) {
  const status = useQuery(syncQueries.status());
  const setUnit = useMutation(syncMutations.setInstanceUnit(id));
  const setKeys = useMutation(syncMutations.setInstanceUnsynced(id));

  const mine = status.data?.find((instance) => instance.id === id);
  const unsynced = mine?.unsynced ?? [];

  const form = useAppForm({
    defaultValues: { key: '' },
    validationLogic: revalidateLogic(),
    validators: {
      onDynamic: z.object({
        key: z.string().min(1, m['instance.sync.keys.required']()),
      }),
    },
    onSubmit: async ({ value, formApi }) => {
      if (!unsynced.includes(value.key)) {
        await setKeys.mutateAsync([...unsynced, value.key]);
      }
      formApi.reset();
    },
  });

  if (status.isPending) return <Bone className="h-24" />;

  const units = mine?.units ?? [];
  if (units.every((unit) => unit.state === 'off')) {
    return (
      <Field>
        <FieldLabel>{m['instance.sync.title']()}</FieldLabel>
        <FieldDescription>
          {m['instance.sync.nothing_shared']()}
        </FieldDescription>
      </Field>
    );
  }

  return (
    <Field>
      <FieldLabel>{m['instance.sync.title']()}</FieldLabel>
      <FieldDescription>{m['instance.sync.description']()}</FieldDescription>

      <div className="divide-y divide-border border border-border">
        {SYNC_UNITS.map((unit) => {
          const state = units.find((entry) => entry.unit === unit)?.state;
          if (!state || state === 'off') return null;
          return (
            <div
              key={unit}
              className="flex items-center gap-3 px-3 py-1.5 text-xs"
            >
              <StatusDot tone={stateTone[state]} />
              <span className="min-w-0 flex-1 truncate">
                {unitLabel[unit]()}
              </span>
              <span className="text-muted-foreground">
                {stateLabel[state]()}
              </span>
              <Switch
                size="sm"
                aria-label={unitLabel[unit]()}
                checked={state !== 'overridden'}
                disabled={setUnit.isPending || state === 'unsupported'}
                onCheckedChange={(checked) =>
                  setUnit.mutate({ unit, shared: checked === true })
                }
              />
            </div>
          );
        })}
      </div>

      <FieldDescription>{m['instance.sync.keys.hint']()}</FieldDescription>
      {unsynced.length > 0 && (
        <div className="divide-y divide-border border border-border">
          {unsynced.map((key) => (
            <ValueRow
              key={key}
              value={key}
              pending={setKeys.isPending}
              onRemove={() =>
                setKeys.mutate(unsynced.filter((other) => other !== key))
              }
            />
          ))}
        </div>
      )}
      <form
        className="flex items-end gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          event.stopPropagation();
          form.handleSubmit();
        }}
      >
        <form.AppField name="key">
          {(field) => (
            <field.TextField
              placeholder={m['instance.sync.keys.placeholder']()}
              className="flex-1"
              inputClassName="font-mono"
            />
          )}
        </form.AppField>
        <form.AppForm>
          <form.SubmitButton>
            {m['instance.sync.keys.action']()}
          </form.SubmitButton>
        </form.AppForm>
      </form>
    </Field>
  );
}
