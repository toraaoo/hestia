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
import { useState } from 'react';

import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
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
import { PickRow } from '@/features/content/pick-row';
import type { ContentProfile, Instance } from '@/features/entries/mock';
import { globalProfiles } from '@/features/profiles/mock';
import { m } from '@/paraglide/messages.js';

/** The content kinds a profile selects over — never datapacks. */
const selectableKinds = ['mod', 'resourcepack', 'shader'] as const;

/**
 * The instance's Profiles tab: named selections over the installed pool, the
 * active one enforced at launch. Local state over the mock — nothing talks to
 * a backend.
 */
export function ProfilesPanel({ inst }: { inst: Instance }) {
  const [profiles, setProfiles] = useState<ContentProfile[]>(inst.profiles);
  const [active, setActive] = useState(inst.activeProfile);
  const [creating, setCreating] = useState(false);
  const [applying, setApplying] = useState(false);
  const [editing, setEditing] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);

  const pool = inst.content.filter((c) =>
    selectableKinds.includes(c.kind as (typeof selectableKinds)[number]),
  );

  const patch = (name: string, change: (p: ContentProfile) => ContentProfile) =>
    setProfiles((list) => list.map((p) => (p.name === name ? change(p) : p)));

  const remove = (name: string) => {
    setProfiles((list) => list.filter((p) => p.name !== name));
    if (active === name) setActive('');
  };

  const rename = (name: string, next: string) => {
    patch(name, (p) => ({ ...p, name: next }));
    if (active === name) setActive(next);
  };

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
              onUse={() =>
                setActive(active === profile.name ? '' : profile.name)
              }
              onEditMembers={() => setEditing(profile.name)}
              onRename={() => setRenaming(profile.name)}
              onCaptureChange={(captured) =>
                patch(profile.name, (p) => ({ ...p, captured }))
              }
              onRemove={() => remove(profile.name)}
            />
          ))}
        </div>
      )}

      <CreateProfileDialog
        open={creating}
        onOpenChange={setCreating}
        taken={profiles.map((p) => p.name)}
        onCreate={(name, seed) =>
          setProfiles((list) => [
            ...list,
            {
              name,
              members: seed ? pool.map((c) => c.id) : [],
              captured: false,
            },
          ])
        }
      />

      <ApplyGlobalDialog
        open={applying}
        onOpenChange={setApplying}
        version={inst.gameVersion}
      />

      <MembersDialog
        profile={profiles.find((p) => p.name === editing) ?? null}
        pool={pool}
        onOpenChange={(open) => !open && setEditing(null)}
        onSave={(name, members) => patch(name, (p) => ({ ...p, members }))}
      />

      <RenameProfileDialog
        name={renaming}
        taken={profiles.map((p) => p.name)}
        onOpenChange={(open) => !open && setRenaming(null)}
        onRename={rename}
      />
    </>
  );
}

function ProfileRow({
  profile,
  poolSize,
  active,
  onUse,
  onEditMembers,
  onRename,
  onCaptureChange,
  onRemove,
}: {
  profile: ContentProfile;
  poolSize: number;
  active: boolean;
  onUse: () => void;
  onEditMembers: () => void;
  onRename: () => void;
  onCaptureChange: (captured: boolean) => void;
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
          <DropdownMenuItem
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
          onCaptureChange(true);
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
          onCaptureChange(false);
        }}
      />
    </div>
  );
}

function CreateProfileDialog({
  open,
  onOpenChange,
  taken,
  onCreate,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  taken: string[];
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
            disabled={invalid}
            onClick={() => {
              onCreate(trimmed, seed);
              close(false);
            }}
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
  onOpenChange,
  onSave,
}: {
  profile: ContentProfile | null;
  pool: Instance['content'];
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
            {pool.map((c) => {
              const checked = members.includes(c.id);
              return (
                <PickRow
                  key={c.id}
                  icon={contentIcon(c.kind)}
                  title={c.name}
                  subtitle={`${contentKindLabel[c.kind]()} · ${c.version}`}
                  selected={checked}
                  onSelect={() =>
                    setSelected(
                      checked
                        ? members.filter((id) => id !== c.id)
                        : [...members, c.id],
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
            onClick={() => {
              if (profile) onSave(profile.name, members);
              close(false);
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
  onOpenChange,
  onRename,
}: {
  name: string | null;
  taken: string[];
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
            disabled={invalid}
            onClick={() => {
              if (name) onRename(name, trimmed);
              close(false);
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
  open,
  onOpenChange,
  version,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  version: string;
}) {
  const [picked, setPicked] = useState<string | null>(null);

  const close = (next: boolean) => {
    if (!next) setPicked(null);
    onOpenChange(next);
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
        {globalProfiles.length === 0 ? (
          <Empty>{m['profiles.global_empty']()}</Empty>
        ) : (
          <div className="grid gap-2 p-1">
            {globalProfiles.map((profile) => (
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
          <Button variant="outline" onClick={() => close(false)}>
            {m['action.cancel']()}
          </Button>
          <Button disabled={picked === null} onClick={() => close(false)}>
            {m['action.apply']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
