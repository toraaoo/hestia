import { DownloadSimpleIcon, PlusIcon } from '@phosphor-icons/react';
import { useQueries } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';

import type { ContentKind, InstanceInfo } from '@/api';
import { Empty } from '@/components/empty';
import { Bone } from '@/components/skeleton';
import { Button } from '@/components/ui/button';
import { m } from '@/paraglide/messages.js';
import {
  instanceQueries,
  useCaptureInstanceProfile,
  useCreateInstanceProfile,
  useEditInstanceProfile,
  useInstanceProfiles,
  useReleaseInstanceProfile,
  useRemoveInstanceProfile,
  useRenameInstanceProfile,
  useUseInstanceProfile,
} from '@/queries/instance';

import { ApplyGlobalDialog } from './dialogs/apply-global';
import { CreateProfileDialog } from './dialogs/create';
import { MembersDialog } from './dialogs/members';
import { RenameProfileDialog } from './dialogs/rename';
import { ProfileRow } from './profile-row';

/** The content kinds a profile selects over — never datapacks. */
const selectableKinds: ContentKind[] = ['mod', 'resource_pack', 'shader'];

const onToastError = {
  onError: (error: Error) => toast.error(error.message),
};

/**
 * The instance's Profiles tab: named selections over the installed pool, the
 * active one enforced at launch. Members are pool filenames.
 */
export function ProfilesPanel({
  instance,
  running,
}: {
  instance: InstanceInfo;
  running: boolean;
}) {
  const id = instance.id;
  const query = useInstanceProfiles(id);
  const create = useCreateInstanceProfile(id);
  const removeProfile = useRemoveInstanceProfile(id);
  const rename = useRenameInstanceProfile(id);
  const use = useUseInstanceProfile(id);
  const edit = useEditInstanceProfile(id);
  const capture = useCaptureInstanceProfile(id);
  const release = useReleaseInstanceProfile(id);

  const [creating, setCreating] = useState(false);
  const [applying, setApplying] = useState(false);
  const [editing, setEditing] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);

  const poolLists = useQueries({
    queries: selectableKinds.map((k) => instanceQueries.content(id, k)),
  });
  const pool = poolLists.flatMap((q) => q.data?.items ?? []);

  if (query.isPending) {
    return (
      <div className="space-y-2">
        <Bone className="h-10" />
        <Bone className="h-10" />
      </div>
    );
  }

  const active = query.data?.active ?? '';
  const profiles = query.data?.profiles ?? [];

  return (
    <>
      <div className="mb-5 flex flex-wrap items-center gap-3">
        <p className="text-sm text-muted-foreground">
          {active
            ? m['profiles.members_count']({
                count:
                  profiles.find((p) => p.name === active)?.members.length ?? 0,
                total: pool.length,
              })
            : m['profiles.none_active']()}
        </p>
        <div className="ml-auto flex items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            data-icon="inline-start"
            disabled={running}
            onClick={() => setApplying(true)}
          >
            <DownloadSimpleIcon weight="bold" />
            {m['profiles.apply_global']()}
          </Button>
          <Button
            size="sm"
            variant="outline"
            data-icon="inline-start"
            onClick={() => setCreating(true)}
          >
            <PlusIcon weight="bold" />
            {m['profiles.new']()}
          </Button>
        </div>
      </div>

      {profiles.length === 0 ? (
        <Empty>{m['profiles.empty']()}</Empty>
      ) : (
        <div className="divide-y divide-border border border-border">
          {profiles.map((profile) => (
            <ProfileRow
              key={profile.name}
              profile={profile}
              poolSize={pool.length}
              active={active === profile.name}
              running={running}
              onUse={() =>
                use.mutate(
                  active === profile.name ? '' : profile.name,
                  onToastError,
                )
              }
              onEditMembers={() => setEditing(profile.name)}
              onRename={() => setRenaming(profile.name)}
              onCapture={() => capture.mutate(profile.name, onToastError)}
              onRelease={() => release.mutate(profile.name, onToastError)}
              onRemove={() => removeProfile.mutate(profile.name, onToastError)}
            />
          ))}
        </div>
      )}

      <CreateProfileDialog
        open={creating}
        onOpenChange={setCreating}
        taken={profiles.map((p) => p.name)}
        pending={create.isPending}
        onCreate={(name, seedFromPool) =>
          create.mutate(
            { name, seedFromPool },
            {
              onSuccess: () => setCreating(false),
              onError: (error) => toast.error(error.message),
            },
          )
        }
      />

      <ApplyGlobalDialog
        instanceId={id}
        open={applying}
        onOpenChange={setApplying}
        version={instance.gameVersion}
      />

      <MembersDialog
        profile={profiles.find((p) => p.name === editing) ?? null}
        pool={pool}
        pending={edit.isPending}
        onOpenChange={(open) => !open && setEditing(null)}
        onSave={(name, members) => {
          const current = profiles.find((p) => p.name === name)?.members ?? [];
          edit.mutate(
            {
              name,
              add: members.filter((f) => !current.includes(f)),
              remove: current.filter((f) => !members.includes(f)),
            },
            {
              onSuccess: () => setEditing(null),
              onError: (error) => toast.error(error.message),
            },
          );
        }}
      />

      <RenameProfileDialog
        name={renaming}
        taken={profiles.map((p) => p.name)}
        pending={rename.isPending}
        onOpenChange={(open) => !open && setRenaming(null)}
        onRename={(name, newName) =>
          rename.mutate(
            { name, newName },
            {
              onSuccess: () => setRenaming(null),
              onError: (error) => toast.error(error.message),
            },
          )
        }
      />
    </>
  );
}
