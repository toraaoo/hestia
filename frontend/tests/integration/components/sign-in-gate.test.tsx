import { screen, waitFor } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { SignInGate } from '@/components/sign-in-gate';
import { player } from '@/mock/channels/account';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries';
import { renderWithProviders, resetQueryCache } from '../../support';

afterEach(resetQueryCache);

describe('SignInGate', () => {
  it('explains what is gated and why', () => {
    renderWithProviders(<SignInGate title="Sign in" hint="Skins need it" />);
    expect(screen.getByRole('heading', { name: 'Sign in' })).toBeDefined();
    expect(screen.getByText('Skins need it')).toBeDefined();
  });

  it('offers the sign-in action', () => {
    renderWithProviders(<SignInGate title="Sign in" hint="Skins need it" />);
    const button = screen.getByRole('button', {
      name: m['account.sign_in'](),
    });
    expect(button.hasAttribute('disabled')).toBe(false);
  });

  it('signs the account in when the action is pressed', async () => {
    function Probe() {
      const { active, signedIn } = useAccounts();
      return <span data-testid="who">{signedIn ? active?.name : '-'}</span>;
    }

    renderWithProviders(
      <>
        <SignInGate title="Sign in" hint="Skins need it" />
        <Probe />
      </>,
    );
    await userEvent.click(
      screen.getByRole('button', { name: m['account.sign_in']() }),
    );

    await waitFor(() =>
      expect(screen.getByTestId('who').textContent).toBe(player.name),
    );
  });
});
