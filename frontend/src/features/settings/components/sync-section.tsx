import { PlusIcon, XIcon } from '@phosphor-icons/react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';

import type { LinkState, SyncTargets } from '@/api';
import { chipClass } from '@/components/chip';
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
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { syncMutations, syncQueries } from '@/queries/sync';

const stateLabel: Record<LinkState, () => string> = {
  linked: () => m['sync.state_linked'](),
  pending: () => m['sync.state_pending'](),
  cannot_link: () => m['sync.state_cannot_link'](),
};

/**
 * The instance-sync settings: the shared target set (files copied, folders
 * linked) and every instance's per-folder link state, with adopt for a
 * non-empty folder the guard refuses to link.
 */
export function SyncSection() {
  const config = useQuery(syncQueries.config());
  const status = useQuery(syncQueries.status());
  const setTargets = useMutation(syncMutations.set());

  const targets = config.data?.targets ?? { files: [], folders: [] };

  const commit = (next: SyncTargets) =>
    setTargets.mutate(next, {
      onError: (error) => toast.error(error.message),
    });

  return (
    <FieldSet>
      <FieldLegend>{m['sync.section']()}</FieldLegend>
      <FieldGroup>
        <FieldDescription>{m['sync.description']()}</FieldDescription>

        {config.isPending ? (
          <div className="space-y-2">
            <Bone className="h-9" />
            <Bone className="h-9" />
          </div>
        ) : (
          <>
            <TargetList
              label={m['sync.files']()}
              placeholder={m['sync.add_file_placeholder']()}
              values={targets.files}
              pending={setTargets.isPending}
              onChange={(files) => commit({ ...targets, files })}
            />
            <TargetList
              label={m['sync.folders']()}
              placeholder={m['sync.add_folder_placeholder']()}
              values={targets.folders}
              pending={setTargets.isPending}
              onChange={(folders) => commit({ ...targets, folders })}
            />
          </>
        )}

        <Field>
          <FieldLabel>{m['sync.status_title']()}</FieldLabel>
          {status.isPending ? (
            <Bone className="h-10" />
          ) : targets.folders.length === 0 ? (
            <p className="text-xs text-muted-foreground">
              {m['sync.no_folder_targets']()}
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
      </FieldGroup>
    </FieldSet>
  );
}

function TargetList({
  label,
  placeholder,
  values,
  pending,
  onChange,
}: {
  label: string;
  placeholder: string;
  values: string[];
  pending: boolean;
  onChange: (values: string[]) => void;
}) {
  const [draft, setDraft] = useState('');

  const add = () => {
    const value = draft.trim();
    if (!value || values.includes(value)) return;
    onChange([...values, value]);
    setDraft('');
  };

  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <div className="flex flex-wrap items-center gap-1.5">
        {values.map((value) => (
          <button
            key={value}
            type="button"
            disabled={pending}
            className={cn(chipClass(true), 'flex items-center gap-1')}
            onClick={() => onChange(values.filter((v) => v !== value))}
          >
            <span className="font-mono">{value}</span>
            <XIcon weight="bold" className="size-3 shrink-0" />
          </button>
        ))}
        <div className="flex items-center gap-1">
          <Input
            value={draft}
            placeholder={placeholder}
            className="h-7 w-40 font-mono text-xs"
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') add();
            }}
          />
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={m['content.add']()}
            disabled={pending || draft.trim().length === 0}
            onClick={add}
          >
            <PlusIcon weight="bold" className="size-3.5" />
          </Button>
        </div>
      </div>
    </Field>
  );
}

function AdoptButton({ id, name }: { id: string; name: string }) {
  const adopt = useMutation(syncMutations.adopt(id));
  return (
    <ConfirmDialog
      trigger={
        <Button variant="outline" size="xs" disabled={adopt.isPending}>
          {m['sync.adopt']()}
        </Button>
      }
      title={`${m['sync.adopt']()} — ${name}`}
      description={m['sync.adopt_description']()}
      confirmLabel={m['sync.adopt']()}
      onConfirm={() =>
        adopt.mutate(undefined, {
          onSuccess: (adopted) =>
            toast.success(m['sync.adopted']({ targets: adopted.join(', ') })),
          onError: (error) => toast.error(error.message),
        })
      }
    />
  );
}
