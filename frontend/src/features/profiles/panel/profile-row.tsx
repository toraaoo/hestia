import {
  CameraIcon,
  CameraSlashIcon,
  DotsThreeIcon,
  PencilSimpleIcon,
  StackIcon,
  TextboxIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { useState } from 'react';

import type { ContentProfile } from '@/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { m } from '@/paraglide/messages.js';

export function ProfileRow({
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
