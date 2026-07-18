import { SignInIcon } from '@phosphor-icons/react';

import { Button } from '@/components/ui/button';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries';

export function SignInGate({ title, hint }: { title: string; hint: string }) {
  const { login } = useAccounts();

  return (
    <div className="grid h-full min-h-full place-items-center px-4">
      <div className="flex max-w-sm flex-col items-center gap-5 text-center">
        <span className="grid size-14 place-items-center bg-muted text-muted-foreground ring-1 ring-border">
          <SignInIcon className="size-7" />
        </span>
        <div className="space-y-1.5">
          <h2 className="text-lg font-medium">{title}</h2>
          <p className="text-sm text-muted-foreground">{hint}</p>
        </div>
        <Button
          data-icon="inline-start"
          disabled={login.isPending}
          onClick={() => login.mutate()}
        >
          <SignInIcon weight="bold" />
          {login.isPending ? m['account.signing_in']() : m['account.sign_in']()}
        </Button>
      </div>
    </div>
  );
}
