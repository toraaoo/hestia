import { createContext, useContext, useEffect, useState } from 'react';

import type { InstanceInfo } from '@/api';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ProvisionProgressView } from '@/features/entries/components/provision-progress';
import { m } from '@/paraglide/messages.js';
import { instanceMutations } from '@/queries/instance';
import { backgroundJob, foregroundJob, useJobMutation } from '@/queries/jobs';

interface LaunchModal {
  launch: (instance: InstanceInfo) => void;
  isLaunching: (id: string) => boolean;
}

const Ctx = createContext<LaunchModal | null>(null);

export function useLaunchModal(): LaunchModal {
  const ctx = useContext(Ctx);
  if (!ctx) {
    throw new Error('useLaunchModal must be used within LaunchModalProvider');
  }
  return ctx;
}

/**
 * Owns the single launch mutation and the first-launch progress modal. An
 * instance that has never been played (`lastPlayedUnix` unset) shows the modal
 * while it materialises; a re-launch runs silently as a backgrounded job. The
 * modal can be dismissed to push the job to the status bar and keep working.
 */
export function LaunchModalProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const mutation = useJobMutation(instanceMutations.launchAny());
  const [target, setTarget] = useState<{ id: string; name: string } | null>(
    null,
  );

  const launch = (instance: InstanceInfo) => {
    mutation.mutate(instance.id);
    if (instance.lastPlayedUnix == null) {
      setTarget({ id: instance.id, name: instance.name });
    }
  };

  const isLaunching = (id: string) =>
    mutation.isPending && mutation.variables === id;

  const job = mutation.job;
  const open = target != null;

  useEffect(() => {
    if (open && job?.status === 'running') foregroundJob(job.id);
  }, [open, job?.id, job?.status]);

  useEffect(() => {
    if (
      target &&
      job &&
      job.status !== 'running' &&
      job.entry?.id === target.id
    ) {
      setTarget(null);
    }
  }, [target, job]);

  const close = () => {
    if (job?.status === 'running') backgroundJob(job.id);
    setTarget(null);
  };

  return (
    <Ctx.Provider value={{ launch, isLaunching }}>
      {children}
      <Dialog
        open={open}
        onOpenChange={(next) => {
          if (!next) close();
        }}
      >
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>
              {m['launch.title']({ name: target?.name ?? '' })}
            </DialogTitle>
            <DialogDescription>{m['launch.preparing']()}</DialogDescription>
          </DialogHeader>
          <ProvisionProgressView
            progress={mutation.progress}
            className="min-h-[18rem] justify-center px-1"
          />
        </DialogContent>
      </Dialog>
    </Ctx.Provider>
  );
}
