import { StackIcon } from '@phosphor-icons/react';
import { useState } from 'react';
import { toast } from 'sonner';
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
import {
  Progress,
  ProgressLabel,
  ProgressValue,
} from '@/components/ui/progress';
import { PickRow } from '@/features/content/components/pick-row';
import { m } from '@/paraglide/messages.js';
import { instanceMutations } from '@/queries/instance';
import { useJobMutation } from '@/queries/jobs';
import { useGlobalProfiles } from '@/queries/profile';

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
  const globals = useGlobalProfiles();
  const apply = useJobMutation(instanceMutations.profiles.apply(instanceId));
  const [picked, setPicked] = useState<string | null>(null);

  const list = globals.data ?? [];
  const progress = apply.progress;
  const percent =
    progress && progress.total > 0
      ? Math.round((progress.current / progress.total) * 100)
      : 0;

  const close = (next: boolean) => {
    if (apply.isPending) return;
    if (!next) setPicked(null);
    onOpenChange(next);
  };

  const run = () => {
    if (!picked) return;
    apply.mutate(picked, {
      onSuccess: (done) => {
        for (const failure of done.failures) toast.error(failure.message);
        setPicked(null);
        onOpenChange(false);
      },
      onError: (error) => toast.error(error.message),
    });
  };

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{m['profiles.apply_title']()}</DialogTitle>
          <DialogDescription>
            {m['profiles.apply_description']({ version })}
          </DialogDescription>
        </DialogHeader>
        {apply.isPending ? (
          <div className="flex min-h-24 flex-col justify-center px-1">
            <Progress value={percent}>
              <ProgressLabel>
                {progress?.detail ||
                  progress?.phase ||
                  m['profiles.apply_global']()}
              </ProgressLabel>
              <ProgressValue />
            </Progress>
          </div>
        ) : list.length === 0 ? (
          <Empty>{m['profiles.global_empty']()}</Empty>
        ) : (
          <div className="grid gap-2 p-1">
            {list.map((profile) => (
              <PickRow
                key={profile.name}
                icon={StackIcon}
                title={profile.name}
                subtitle={m['profiles.entries_count']({
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
            {m['action.cancel']()}
          </Button>
          <Button disabled={picked === null || apply.isPending} onClick={run}>
            {m['action.apply']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
