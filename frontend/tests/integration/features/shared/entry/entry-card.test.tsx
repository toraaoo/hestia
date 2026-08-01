import { screen, waitFor, within } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  type EntryCardModel,
  EntryTile,
} from '@/features/shared/entry/components';
import { m } from '@/paraglide/messages.js';
import { renderWithProviders, resetQueryCache } from '../../../../support';

const entry = (over: Partial<EntryCardModel> = {}): EntryCardModel => ({
  id: 'smp',
  name: 'SMP',
  kind: 'server',
  flavor: 'paper',
  version: '1.21.1',
  running: false,
  ready: true,
  subtitle: ':25565 · Stopped',
  ...over,
});

const show = (model: EntryCardModel) =>
  renderWithProviders(<EntryTile entry={model} view="grid" />, { route: true });

afterEach(resetQueryCache);

describe('EntryTile (grid)', () => {
  it('shows the entry’s identity', async () => {
    show(entry());
    expect(await screen.findByText('SMP')).toBeDefined();
    expect(screen.getByText('paper')).toBeDefined();
    expect(screen.getByText('1.21.1')).toBeDefined();
    expect(screen.getByText(':25565 · Stopped')).toBeDefined();
  });

  it('links to the entry the card is for', async () => {
    show(entry());
    await screen.findByText('SMP');
    expect(
      screen.getByRole('link').getAttribute('href'),
    ).toBe('/servers/smp');
  });

  it('links an instance to the instance route instead', async () => {
    show(entry({ kind: 'instance', id: 'modded', name: 'Modded' }));
    await screen.findByText('Modded');
    expect(screen.getByRole('link').getAttribute('href')).toBe(
      '/instances/modded',
    );
  });

  it('offers start on a stopped server and play on an instance', async () => {
    show(entry());
    expect(
      await screen.findByRole('button', { name: m['app.action.start']() }),
    ).toBeDefined();

    resetQueryCache();
    show(entry({ kind: 'instance', id: 'modded', name: 'Modded' }));
    expect(
      await screen.findByRole('button', { name: m['app.action.play']() }),
    ).toBeDefined();
  });

  it('starts the entry when the action is pressed', async () => {
    const onStart = vi.fn();
    show(entry({ onStart }));
    const button = await screen.findByRole('button', {
      name: m['app.action.start'](),
    });
    await userEvent.click(button);
    expect(onStart).toHaveBeenCalledOnce();
  });

  it('refuses to start an entry that is not provisioned yet', async () => {
    show(entry({ ready: false }));
    const button = await screen.findByRole('button', {
      name: m['app.action.start'](),
    });
    expect(button.hasAttribute('disabled')).toBe(true);
    expect(screen.getByText(m['app.status.preparing']())).toBeDefined();
  });

  it('refuses to start again while a start is in flight', async () => {
    show(entry({ busy: true }));
    const button = await screen.findByRole('button', {
      name: new RegExp(m['app.action.start']()),
    });
    expect(button.hasAttribute('disabled')).toBe(true);
    expect(screen.getByRole('status')).toBeDefined();
  });

  it('offers stop, and confirms before stopping, while running', async () => {
    const onStop = vi.fn();
    show(entry({ running: true, onStop }));

    await userEvent.click(
      await screen.findByRole('button', { name: m['app.action.stop']() }),
    );
    expect(onStop).not.toHaveBeenCalled();

    const dialog = await screen.findByRole('alertdialog');
    await userEvent.click(
      within(dialog).getByRole('button', { name: m['app.action.stop']() }),
    );
    await waitFor(() => expect(onStop).toHaveBeenCalledOnce());
  });

  it('marks a running server online and a running instance running', async () => {
    show(entry({ running: true }));
    expect(await screen.findByText(m['app.status.online']())).toBeDefined();

    resetQueryCache();
    show(entry({ kind: 'instance', id: 'modded', running: true }));
    expect(await screen.findByText(m['app.status.running']())).toBeDefined();
  });
});

describe('EntryTile (list)', () => {
  it('renders the same entry as one line', async () => {
    renderWithProviders(<EntryTile entry={entry()} view="list" />, {
      route: true,
    });
    expect(await screen.findByText('SMP')).toBeDefined();
    expect(
      screen.getByText('paper · 1.21.1 · :25565 · Stopped'),
    ).toBeDefined();
  });
});
