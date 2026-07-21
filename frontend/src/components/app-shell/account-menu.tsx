import { CaretUpDownIcon, PlusIcon, SignOutIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import { AccountAvatar } from '@/components/app-shell/account-avatar';
import { Bone } from '@/components/skeleton';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries/accounts';

export function AccountMenu() {
  const {
    accounts,
    active,
    isPending,
    login,
    switch: switchAccount,
    remove: removeAccount,
  } = useAccounts();
  const [signingOut, setSigningOut] = useState(false);

  const others = active
    ? accounts.filter((a) => a.uuid !== active.uuid)
    : accounts;

  if (isPending) {
    return (
      <div className="flex w-full items-center gap-2.5 px-3 py-2">
        <Bone className="size-7 shrink-0" />
        <span className="min-w-0 flex-1 space-y-1.5">
          <Bone className="h-3.5 w-24" />
          <Bone className="h-2.5 w-16" />
        </span>
      </div>
    );
  }

  if (!active) {
    return (
      <button
        type="button"
        disabled={login.isPending}
        onClick={() => login.mutate()}
        className="flex w-full items-center gap-2.5 px-3 py-2 text-left transition-colors outline-none hover:bg-muted focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset disabled:opacity-60"
      >
        <span className="grid size-7 shrink-0 place-items-center bg-muted text-muted-foreground ring-1 ring-border">
          <PlusIcon className="size-4" />
        </span>
        <span className="min-w-0 flex-1">
          <span className="block truncate text-sm">
            {login.isPending
              ? m['account.signing_in']()
              : m['account.sign_in']()}
          </span>
          <span className="block truncate text-[11px] text-muted-foreground">
            {login.isError
              ? m['account.sign_in_failed']()
              : m['account.not_signed_in']()}
          </span>
        </span>
      </button>
    );
  }

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <button
              type="button"
              className="flex w-full items-center gap-2.5 px-3 py-2 text-left transition-colors outline-none hover:bg-muted focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset aria-expanded:bg-muted"
            >
              <AccountAvatar
                uuid={active.uuid}
                name={active.name}
                size={28}
                className="text-[11px]"
              />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm">{active.name}</span>
                <span className="block truncate text-[11px] text-muted-foreground">
                  {login.isPending
                    ? m['account.signing_in']()
                    : m['account.microsoft']()}
                </span>
              </span>
              <CaretUpDownIcon className="size-4 shrink-0 text-muted-foreground" />
            </button>
          }
        />
        <DropdownMenuContent side="top" align="start" className="w-48">
          <DropdownMenuGroup>
            <DropdownMenuLabel>
              {m['account.signed_in_as']({ name: active.name })}
            </DropdownMenuLabel>
            {others.map((a) => (
              <DropdownMenuItem
                key={a.uuid}
                onClick={() => switchAccount.mutate(a.uuid)}
              >
                <AccountAvatar
                  uuid={a.uuid}
                  name={a.name}
                  size={20}
                  className="text-[9px]"
                />
                {m['account.switch_to']({ name: a.name })}
              </DropdownMenuItem>
            ))}
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuItem
              disabled={login.isPending}
              onClick={() => login.mutate()}
            >
              <PlusIcon />
              {m['account.add']()}
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => setSigningOut(true)}>
              <SignOutIcon />
              {m['account.sign_out']()}
            </DropdownMenuItem>
          </DropdownMenuGroup>
        </DropdownMenuContent>
      </DropdownMenu>

      <ConfirmDialog
        open={signingOut}
        onOpenChange={setSigningOut}
        title={m['account.sign_out_title']({ name: active.name })}
        description={m['account.sign_out_description']()}
        destructive
        confirmLabel={m['account.sign_out']()}
        onConfirm={() => {
          removeAccount.mutate(active.uuid);
          setSigningOut(false);
        }}
      />
    </>
  );
}
