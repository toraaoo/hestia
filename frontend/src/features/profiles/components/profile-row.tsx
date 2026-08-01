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

import type { Profile } from '@/api';
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
  profile: Profile;
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
              {m['profile.active']()}
            </Badge>
          )}
          {profile.captured && (
            <Badge variant="secondary" className="shrink-0 gap-1">
              <CameraIcon className="size-3" />
              {m['profile.capture.badge']()}
            </Badge>
          )}
        </div>
        <div className="truncate font-mono text-[11px] text-muted-foreground">
          {m['profile.members.count']({
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
        {active ? m['app.action.deactivate']() : m['profile.use']()}
      </Button>
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={m['app.action.more']()}
            >
              <DotsThreeIcon weight="bold" className="size-4" />
            </Button>
          }
        />
        <DropdownMenuContent align="end" className="w-52">
          <DropdownMenuItem onClick={onEditMembers}>
            <PencilSimpleIcon />
            {m['profile.members.action']()}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={onRename}>
            <TextboxIcon />
            {m['app.action.rename']()}
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
              ? m['profile.release.action']()
              : m['profile.capture.action']()}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            variant="destructive"
            onClick={() => setConfirming('remove')}
          >
            <TrashIcon />
            {m['app.action.remove']()}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <ConfirmDialog
        open={confirming === 'remove'}
        onOpenChange={(open) => !open && setConfirming(null)}
        title={m['profile.remove.title']({ name: profile.name })}
        description={m['profile.remove.description']()}
        destructive
        confirmLabel={m['app.action.remove']()}
        onConfirm={() => {
          setConfirming(null);
          onRemove();
        }}
      />
      <ConfirmDialog
        open={confirming === 'capture'}
        onOpenChange={(open) => !open && setConfirming(null)}
        title={m['profile.capture.title']({ name: profile.name })}
        description={m['profile.capture.description']()}
        confirmLabel={m['profile.capture.action']()}
        onConfirm={() => {
          setConfirming(null);
          onCapture();
        }}
      />
      <ConfirmDialog
        open={confirming === 'release'}
        onOpenChange={(open) => !open && setConfirming(null)}
        title={m['profile.release.title']()}
        description={m['profile.release.description']()}
        destructive
        confirmLabel={m['profile.release.action']()}
        onConfirm={() => {
          setConfirming(null);
          onRelease();
        }}
      />
    </div>
  );
}
