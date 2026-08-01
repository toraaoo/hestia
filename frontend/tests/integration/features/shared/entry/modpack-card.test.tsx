import { screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { InstalledModpack } from '@/api';
import { ModpackCard } from '@/features/shared/entry/detail';
import { clearPack, listInstances, setPack } from '@/mock/state/entries';
import { m } from '@/paraglide/messages.js';
import { renderWithProviders, resetQueryCache } from '../../../../support';

const instance = listInstances()[0];

const pack = (over: Partial<InstalledModpack> = {}): InstalledModpack =>
  ({
    source: 'modrinth',
    projectId: 'fabulously-optimized',
    slug: 'fabulously-optimized',
    name: 'Fabulously Optimized',
    versionId: 'v0',
    versionNumber: '1.4.0',
    gameVersion: '1.21.1',
    loader: 'fabric',
    loaderVersion: '0.16.5',
    installedUnix: 0,
    files: [],
    overrides: [],
    ...over,
  }) as InstalledModpack;

const card = (running: boolean) =>
  renderWithProviders(
    <ModpackCard
      kind="instance"
      id={instance.id}
      name={instance.name}
      running={running}
    />,
  );

const updateButton = () =>
  screen.getByRole('button', { name: m['content.modpack.update']() });
const removeButton = () =>
  screen.getByRole('button', { name: m['content.modpack.remove.action']() });

beforeEach(() => setPack(instance.id, pack()));
afterEach(() => {
  clearPack(instance.id);
  resetQueryCache();
});

describe('the modpack card', () => {
  it('offers update and remove while the entry is idle', async () => {
    card(false);
    await waitFor(() => expect(updateButton()).toBeDefined());
    expect(updateButton().hasAttribute('disabled')).toBe(false);
    expect(removeButton().hasAttribute('disabled')).toBe(false);
  });

  it('refuses both while the entry is running', async () => {
    card(true);
    await waitFor(() => expect(updateButton()).toBeDefined());
    expect(updateButton().hasAttribute('disabled')).toBe(true);
    expect(removeButton().hasAttribute('disabled')).toBe(true);
  });

  it('hides update for a pack with no catalogue behind it', async () => {
    setPack(instance.id, pack({ projectId: '' }));
    card(false);
    await waitFor(() => expect(removeButton()).toBeDefined());
    expect(
      screen.queryByRole('button', { name: m['content.modpack.update']() }),
    ).toBeNull();
  });

  it('says so plainly when the entry was not built from a pack', async () => {
    clearPack(instance.id);
    card(false);
    await waitFor(() =>
      expect(screen.getByText(m['content.modpack.none']())).toBeDefined(),
    );
  });
});
