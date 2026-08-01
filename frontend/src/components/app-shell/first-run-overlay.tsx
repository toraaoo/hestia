import { Logo } from '@/components/app-shell/logo';
import { Button } from '@/components/ui/button';
import { m } from '@/paraglide/messages.js';
import { useAccounts, useConnection, usePrefs } from '@/queries';

const DISMISS_KEY = 'welcome-dismissed';

export function FirstRunOverlay() {
  const { signedIn, ready: accountsReady, login, signingIn } = useAccounts();
  const { get, set, ready: prefsReady } = usePrefs();
  const connected = useConnection() === 'connected';

  // Offline, "not signed in" is unknown rather than true, and the offline
  // overlay owns the screen.
  if (!connected || !accountsReady || !prefsReady || signedIn) return null;
  if (get(DISMISS_KEY, false)) return null;

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-background/80 backdrop-blur-xs">
      <div className="flex w-full max-w-md flex-col items-center gap-6 border border-border bg-card px-8 py-10 text-center shadow-lg">
        <Logo className="size-12" />
        <div className="space-y-2">
          <h2 className="text-xl font-semibold">{m['app.welcome.title']()}</h2>
          <p className="text-sm text-muted-foreground">
            {m['app.welcome.body']()}
          </p>
        </div>
        <div className="flex w-full flex-col gap-2">
          <Button disabled={signingIn} onClick={() => login.mutate()}>
            {signingIn ? m['account.signing_in']() : m['account.sign_in']()}
          </Button>
          <Button variant="ghost" onClick={() => set(DISMISS_KEY, true)}>
            {m['app.welcome.dismiss']()}
          </Button>
        </div>
      </div>
    </div>
  );
}
