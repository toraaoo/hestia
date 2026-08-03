import { StackIcon } from '@phosphor-icons/react';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';
import { errorMessageFromInfo } from '@/api';
import { Empty } from '@/components/empty';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { PickRow } from '@/features/shared/content/components';
import { ProvisionProgressView } from '@/features/shared/entry/components';
import { m } from '@/paraglide/messages.js';
import { instanceMutations } from '@/queries/instance';
import { useJobDisplay, useJobMutation } from '@/queries/jobs';
import { profileQueries } from '@/queries/profile';

export function ApplyGlobalDialog({
  instanceId,
  open,
  onOpenChange,
  version,
}: {
  instanceId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  version: string;
}) {
  const globals = useQuery(profileQueries.list());
  const apply = useJobMutation(instanceMutations.profiles.apply(instanceId));
  useJobDisplay(apply.job, open);
  const [picked, setPicked] = useState<string | null>(null);

  const list = globals.data ?? [];
  const progress = apply.progress;

  const close = (next: boolean) => {
    if (apply.isPending) return;
    if (!next) setPicked(null);
    onOpenChange(next);
  };

  const run = () => {
    if (!picked) return;
    apply.mutate(picked, {
      onSuccess: (done) => {
        for (const failure of done.failures)
          toast.error(errorMessageFromInfo(failure.error));
        setPicked(null);
        onOpenChange(false);
      },
    });
  };

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{m['profile.apply.title']()}</DialogTitle>
          <DialogDescription>
            {m['profile.apply.description']({ version })}
          </DialogDescription>
        </DialogHeader>
        {apply.isPending ? (
          <ProvisionProgressView
            progress={progress ?? null}
            fallbackLabel={m['profile.apply.action']()}
            className="min-h-24"
          />
        ) : list.length === 0 ? (
          <Empty icon={StackIcon}>{m['profile.global.empty']()}</Empty>
        ) : (
          <div className="grid gap-2 p-1">
            {list.map((profile) => (
              <PickRow
                key={profile.name}
                icon={StackIcon}
                title={profile.name}
                subtitle={m['profile.global.entries_count']({
                  count: profile.entries.length,
                })}
                selected={picked === profile.name}
                onSelect={() => setPicked(profile.name)}
              />
            ))}
          </div>
        )}
        <DialogFooter>
          <Button
            variant="outline"
            disabled={apply.isPending}
            onClick={() => close(false)}
          >
            {m['app.action.cancel']()}
          </Button>
          <Button disabled={picked === null || apply.isPending} onClick={run}>
            {m['app.action.apply']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
