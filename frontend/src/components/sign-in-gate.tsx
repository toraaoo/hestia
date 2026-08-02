import { SignInIcon } from '@phosphor-icons/react';

import { Empty } from '@/components/empty';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries';

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

  return (
    <Empty
      icon={SignInIcon}
      description={hint}
      className={cn('h-full', className)}
      action={
        <Button
          data-icon="inline-start"
          disabled={signingIn}
          onClick={() => login.mutate()}
        >
          <SignInIcon weight="bold" />
          {signingIn ? m['account.signing_in']() : m['account.sign_in']()}
        </Button>
      }
    >
      {title}
    </Empty>
  );
}
