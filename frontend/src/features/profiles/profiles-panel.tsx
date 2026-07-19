import {
  CameraIcon,
  CameraSlashIcon,
  DotsThreeIcon,
  DownloadSimpleIcon,
  PencilSimpleIcon,
  PlusIcon,
  StackIcon,
  TextboxIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { useQueries } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';

import type {
  ContentKind,
  ContentProfile,
  InstalledContent,
  InstanceInfo,
} from '@/api';
import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Progress,
  ProgressLabel,
  ProgressValue,
} from '@/components/ui/progress';
import { PickRow } from '@/features/content/pick-row';
import { m } from '@/paraglide/messages.js';
import {
  instanceQueries,
  useApplyInstanceProfile,
  useCaptureInstanceProfile,
  useCreateInstanceProfile,
  useEditInstanceProfile,
  useInstanceProfiles,
  useReleaseInstanceProfile,
  useRemoveInstanceProfile,
  useRenameInstanceProfile,
  useUseInstanceProfile,
} from '@/queries/instance';
import { useGlobalProfiles } from '@/queries/profile';

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

function ProfileRow({
  profile,
  poolSize,
  active,
  running,
  onUse,
  onEditMembers,
  onRename,
  onCapture,
  onRelease,
  onRemove,
}: {
  profile: ContentProfile;
  poolSize: number;
  active: boolean;
  running: boolean;
  onUse: () => void;
  onEditMembers: () => void;
  onRename: () => void;
  onCapture: () => void;
  onRelease: () => void;
  onRemove: () => void;
}) {
  const [confirming, setConfirming] = useState<
    'remove' | 'capture' | 'release' | null
  >(null);

  return (
    <div className="flex items-center gap-3 px-3 py-2.5">
      <StackIcon
        weight={active ? 'fill' : 'regular'}
        className={
          active
            ? 'size-4 shrink-0 text-ember'
            : 'size-4 shrink-0 text-muted-foreground'
        }
      />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm">{profile.name}</span>
          {active && (
            <Badge className="shrink-0 bg-ember text-ember-foreground">
              {m['profiles.active']()}
            </Badge>
          )}
          {profile.captured && (
            <Badge variant="secondary" className="shrink-0 gap-1">
              <CameraIcon className="size-3" />
              {m['profiles.captured']()}
            </Badge>
          )}
        </div>
        <div className="truncate font-mono text-[11px] text-muted-foreground">
          {m['profiles.members_count']({
            count: profile.members.length,
            total: poolSize,
          })}
        </div>
      </div>
      <Button
        size="sm"
        variant={active ? 'secondary' : 'outline'}
        onClick={onUse}
      >
        {active ? m['profiles.deactivate']() : m['profiles.use']()}
      </Button>
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={m['action.more']()}
            >
              <DotsThreeIcon weight="bold" className="size-4" />
            </Button>
          }
        />
        <DropdownMenuContent align="end" className="w-52">
          <DropdownMenuItem onClick={onEditMembers}>
            <PencilSimpleIcon />
            {m['profiles.edit_members']()}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={onRename}>
            <TextboxIcon />
            {m['profiles.rename']()}
          </DropdownMenuItem>
          {/* Capture/release move the profile's settings store — the daemon
              refuses them while a session could be writing through it. */}
          <DropdownMenuItem
            disabled={running}
            onClick={() =>
              setConfirming(profile.captured ? 'release' : 'capture')
            }
          >
            {profile.captured ? <CameraSlashIcon /> : <CameraIcon />}
            {profile.captured
              ? m['profiles.release']()
              : m['profiles.capture']()}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            variant="destructive"
            onClick={() => setConfirming('remove')}
          >
            <TrashIcon />
            {m['action.remove']()}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <ConfirmDialog
        open={confirming === 'remove'}
        onOpenChange={(open) => !open && setConfirming(null)}
        title={m['profiles.remove_title']({ name: profile.name })}
        description={m['profiles.remove_description']()}
        destructive
        confirmLabel={m['action.remove']()}
        onConfirm={() => {
          setConfirming(null);
          onRemove();
        }}
      />
      <ConfirmDialog
        open={confirming === 'capture'}
        onOpenChange={(open) => !open && setConfirming(null)}
        title={m['profiles.capture_title']({ name: profile.name })}
        description={m['profiles.capture_description']()}
        confirmLabel={m['profiles.capture']()}
        onConfirm={() => {
          setConfirming(null);
          onCapture();
        }}
      />
      <ConfirmDialog
        open={confirming === 'release'}
        onOpenChange={(open) => !open && setConfirming(null)}
        title={m['profiles.release_title']()}
        description={m['profiles.release_description']()}
        destructive
        confirmLabel={m['profiles.release']()}
        onConfirm={() => {
          setConfirming(null);
          onRelease();
        }}
      />
    </div>
  );
}

function CreateProfileDialog({
  open,
  onOpenChange,
  taken,
  pending,
  onCreate,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  taken: string[];
  pending: boolean;
  onCreate: (name: string, seedFromPool: boolean) => void;
}) {
  const [name, setName] = useState('');
  const [seed, setSeed] = useState(true);
  const trimmed = name.trim();
  const invalid =
    trimmed.length === 0 ||
    trimmed.toLowerCase() === 'none' ||
    taken.some((t) => t.toLowerCase() === trimmed.toLowerCase());

  const close = (next: boolean) => {
    if (!next) {
      setName('');
      setSeed(true);
    }
    onOpenChange(next);
  };

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>{m['profiles.create_title']()}</DialogTitle>
          <DialogDescription>
            {m['profiles.create_description']()}
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4">
          <Field>
            <FieldLabel>{m['profiles.name_label']()}</FieldLabel>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
            />
          </Field>
          <label
            htmlFor="profile-seed"
            className="flex items-start gap-2.5 text-sm"
          >
            <Checkbox
              id="profile-seed"
              checked={seed}
              onCheckedChange={(checked) => setSeed(checked === true)}
              className="mt-0.5"
            />
            <span>
              {m['profiles.seed_label']()}
              <span className="block text-[11px] text-muted-foreground">
                {m['profiles.seed_hint']()}
              </span>
            </span>
          </label>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => close(false)}>
            {m['action.cancel']()}
          </Button>
          <Button
            disabled={invalid || pending}
            onClick={() => onCreate(trimmed, seed)}
          >
            {m['action.confirm']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function MembersDialog({
  profile,
  pool,
  pending,
  onOpenChange,
  onSave,
}: {
  profile: ContentProfile | null;
  pool: InstalledContent[];
  pending: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (name: string, members: string[]) => void;
}) {
  const [selected, setSelected] = useState<string[] | null>(null);
  const members = selected ?? profile?.members ?? [];

  const close = (next: boolean) => {
    if (!next) setSelected(null);
    onOpenChange(next);
  };

  return (
    <Dialog open={profile !== null} onOpenChange={close}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {m['profiles.members_title']({ name: profile?.name ?? '' })}
          </DialogTitle>
          <DialogDescription>
            {m['profiles.members_description']()}
          </DialogDescription>
        </DialogHeader>
        {pool.length === 0 ? (
          <Empty>{m['profiles.members_empty']()}</Empty>
        ) : (
          <div className="grid max-h-72 gap-2 overflow-y-auto p-1">
            {pool.map((item) => {
              const checked = members.includes(item.filename);
              return (
                <PickRow
                  key={item.filename}
                  icon={contentIcon(item.kind)}
                  title={item.title || item.filename}
                  subtitle={`${contentKindLabel[item.kind]()} · ${item.versionNumber || item.filename}`}
                  selected={checked}
                  onSelect={() =>
                    setSelected(
                      checked
                        ? members.filter((f) => f !== item.filename)
                        : [...members, item.filename],
                    )
                  }
                />
              );
            })}
          </div>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={() => close(false)}>
            {m['action.cancel']()}
          </Button>
          <Button
            disabled={pending}
            onClick={() => {
              if (profile) onSave(profile.name, members);
            }}
          >
            {m['action.apply']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function RenameProfileDialog({
  name,
  taken,
  pending,
  onOpenChange,
  onRename,
}: {
  name: string | null;
  taken: string[];
  pending: boolean;
  onOpenChange: (open: boolean) => void;
  onRename: (name: string, next: string) => void;
}) {
  const [value, setValue] = useState('');
  const trimmed = value.trim();
  const invalid =
    trimmed.length === 0 ||
    trimmed.toLowerCase() === 'none' ||
    taken.some(
      (t) =>
        t.toLowerCase() === trimmed.toLowerCase() &&
        t.toLowerCase() !== name?.toLowerCase(),
    );

  const close = (next: boolean) => {
    if (!next) setValue('');
    onOpenChange(next);
  };

  return (
    <Dialog open={name !== null} onOpenChange={close}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>{m['profiles.rename_title']()}</DialogTitle>
        </DialogHeader>
        <Field>
          <FieldLabel>{m['profiles.name_label']()}</FieldLabel>
          <Input
            value={value}
            placeholder={name ?? ''}
            onChange={(e) => setValue(e.target.value)}
            autoFocus
          />
        </Field>
        <DialogFooter>
          <Button variant="outline" onClick={() => close(false)}>
            {m['action.cancel']()}
          </Button>
          <Button
            disabled={invalid || pending}
            onClick={() => {
              if (name) onRename(name, trimmed);
            }}
          >
            {m['profiles.rename']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ApplyGlobalDialog({
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
  const apply = useApplyInstanceProfile(instanceId);
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
