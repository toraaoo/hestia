import { SignInIcon } from '@phosphor-icons/react';

import { Empty } from '@/components/empty';
import { OfflineNotice } from '@/components/offline-state';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries';
import { useOffline } from '@/queries/net';

export function SignInGate({
  title,
  hint,
  className,
}: {
  title: string;
  hint: string;
  className?: string;
}) {
  const { login, signingIn } = useAccounts();
  const offline = useOffline();

  return (
    <Empty
      icon={SignInIcon}
      description={hint}
      className={cn('h-full', className)}
      action={
        <div className="flex flex-col items-center gap-2">
          <Button
            data-icon="inline-start"
            disabled={signingIn || offline}
            onClick={() => login.mutate()}
          >
            <SignInIcon weight="bold" />
            {signingIn ? m['account.signing_in']() : m['account.sign_in']()}
          </Button>
          {/* Signing in is a round trip to Microsoft; there is nothing local
              that could stand in for it. */}
          {offline && <OfflineNotice />}
        </div>
      }
    >
      {title}
    </Empty>
  );
}
