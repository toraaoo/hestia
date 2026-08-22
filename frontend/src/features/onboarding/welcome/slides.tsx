import {
  CheckCircleIcon,
  CompassIcon,
  HandWavingIcon,
  type Icon,
  SignInIcon,
} from '@phosphor-icons/react';
import type { ReactNode } from 'react';

import { OfflineNotice } from '@/components/offline-state';
import { Button } from '@/components/ui/button';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries';
import { useOffline } from '@/queries/net';

export interface WelcomeSlide {
  id: string;
  icon: Icon;
  title: () => string;
  body: () => string;
  Extra?: () => ReactNode;
}

export const slides: readonly WelcomeSlide[] = [
  {
    id: 'hello',
    icon: HandWavingIcon,
    title: m['onboarding.welcome.hello.title'],
    body: m['onboarding.welcome.hello.body'],
  },
  {
    id: 'account',
    icon: SignInIcon,
    title: m['onboarding.welcome.account.title'],
    body: m['onboarding.welcome.account.body'],
    Extra: SignIn,
  },
  {
    id: 'ready',
    icon: CompassIcon,
    title: m['onboarding.welcome.ready.title'],
    body: m['onboarding.welcome.ready.body'],
  },
];

function SignIn() {
  const { signedIn, signingIn, login } = useAccounts();
  const offline = useOffline();

  if (signedIn) {
    return (
      <p className="flex items-center justify-center gap-1.5 text-xs text-muted-foreground">
        <CheckCircleIcon weight="fill" className="size-4 text-ember" />
        {m['onboarding.welcome.account.signed_in']()}
      </p>
    );
  }

  return (
    <div className="flex flex-col items-center gap-2">
      <Button
        data-icon="inline-start"
        disabled={signingIn || offline}
        onClick={() => login.mutate()}
      >
        <SignInIcon weight="bold" />
        {signingIn ? m['account.signing_in']() : m['account.sign_in']()}
      </Button>
      {offline && <OfflineNotice />}
    </div>
  );
}
