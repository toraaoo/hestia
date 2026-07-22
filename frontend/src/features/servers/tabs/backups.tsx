import { PlusIcon, TrashIcon } from '@phosphor-icons/react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';

import type { BackupInfo, BackupKind, ConfigEntry } from '@/api';
import { Empty } from '@/components/empty';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { agoLabel, bytes } from '@/lib/format';
import { m } from '@/paraglide/messages.js';
import { useJobMutation } from '@/queries/jobs';
import { serverMutations, serverQueries } from '@/queries/server';

function kindLabel(kind: BackupKind): string {
  switch (kind) {
    case 'manual':
      return m['backup.kind_manual']();
    case 'scheduled':
      return m['backup.kind_scheduled']();
    case 'update':
      return m['backup.kind_update']();
  }
}

export function ServerBackupsTab({
  id,
  running,
  config,
}: {
  id: string;
  running: boolean;
  config?: ConfigEntry[];
}) {
  const backups = useQuery(serverQueries.backups(id));
  const create = useJobMutation(serverMutations.backup.create(id));
  const restore = useJobMutation(serverMutations.backup.restore(id));
  const remove = useMutation(serverMutations.backup.remove(id));

  const interval = config?.find((e) => e.key === 'backup-interval')?.value;
  const retention = config?.find((e) => e.key === 'backup-retention')?.value;

  const list = backups.data ?? [];

  return (
    <>
      <div className="mb-5 flex items-center justify-between">
        <span className="text-xs text-muted-foreground">
          {interval
            ? m['backup.schedule_status']({
                interval,
                retention: Number(retention ?? 0),
              })
            : m['backup.off_short']()}
        </span>
        <Button
          size="sm"
          variant="outline"
          data-icon="inline-start"
          disabled={create.isPending}
          onClick={() =>
            create.mutate(undefined, {
              onSuccess: () => toast.success(m['toast.saved']()),
            })
          }
        >
          <PlusIcon weight="bold" />
          {m['backup.create']()}
        </Button>
      </div>

      {list.length === 0 ? (
        <Empty>{m['backup.none']()}</Empty>
      ) : (
        <div className="divide-y divide-border border border-border">
          {list.map((backup: BackupInfo) => (
            <div
              key={backup.id}
              className="flex items-center gap-3 px-3 py-2.5"
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="text-sm">
                    {agoLabel(backup.createdUnix)}
                  </span>
                  <Badge variant="secondary" className="shrink-0 capitalize">
                    {kindLabel(backup.kind)}
                  </Badge>
                </div>
                <div className="font-mono text-[11px] text-muted-foreground">
                  {bytes(backup.size)}
                </div>
              </div>
              <ConfirmDialog
                trigger={
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={running || restore.isPending}
                  >
                    {m['action.restore']()}
                  </Button>
                }
                title={m['backup.restore_title']()}
                description={m['backup.restore_description']({
                  when: agoLabel(backup.createdUnix),
                })}
                confirmLabel={m['action.restore']()}
                onConfirm={() =>
                  restore.mutate(backup.id, {
                    onSuccess: () => toast.success(m['toast.saved']()),
                  })
                }
              />
              <ConfirmDialog
                trigger={
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={m['backup.delete_aria']()}
                  >
                    <TrashIcon className="size-4" />
                  </Button>
                }
                title={m['backup.delete_title']()}
                description={m['backup.delete_description']({
                  when: agoLabel(backup.createdUnix),
                })}
                destructive
                confirmLabel={m['action.delete']()}
                onConfirm={() => remove.mutate(backup.id)}
              />
            </div>
          ))}
        </div>
      )}
    </>
  );
}
