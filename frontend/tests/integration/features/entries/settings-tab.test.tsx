import { screen, waitFor } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ConfigEntry } from '@/api';
import { InstanceSettingsTab } from '@/features/instances/tabs/settings';
import { ServerSettingsTab } from '@/features/servers/tabs/settings';
import { listInstances, listServers } from '@/mock/state/entries';
import { m } from '@/paraglide/messages.js';
import { renderWithProviders, resetQueryCache } from '../../../support';

const server = listServers()[0];
const instance = listInstances()[0];

const config = (over: Record<string, string> = {}): ConfigEntry[] =>
  Object.entries({ memory: '6G', 'jvm-args': '-XX:+UseZGC', ...over }).map(
    ([key, value]) => ({ key, value }),
  );

const showServer = (running = false) =>
  renderWithProviders(
    <ServerSettingsTab server={server} config={config()} running={running} />,
    { route: true },
  );

const showInstance = (running = false) =>
  renderWithProviders(
    <InstanceSettingsTab
      instance={instance}
      config={config()}
      running={running}
    />,
    { route: true },
  );

afterEach(() => {
  resetQueryCache();
  vi.restoreAllMocks();
});

describe('the shared entry settings tab', () => {
  it('seeds the name and JVM args from the entry and its config', async () => {
    showServer();
    const name = (await screen.findByLabelText(
      m['entry.settings.server_name'](),
    )) as HTMLInputElement;
    expect(name.value).toBe(server.name);

    const args = screen.getByLabelText(
      m['entry.settings.java_arguments'](),
    ) as HTMLInputElement;
    expect(args.value).toBe('-XX:+UseZGC');
  });

  it('labels the name field for the kind it is showing', async () => {
    showInstance();
    expect(
      await screen.findByLabelText(m['entry.settings.instance_name']()),
    ).toBeDefined();
    expect(
      screen.queryByLabelText(m['entry.settings.server_name']()),
    ).toBeNull();
  });

  it('keeps rename inert until the name actually changes', async () => {
    showServer();
    const name = await screen.findByLabelText(
      m['entry.settings.server_name'](),
    );
    const [apply] = screen.getAllByRole('button', {
      name: m['app.action.apply'](),
    });
    expect(apply.hasAttribute('disabled')).toBe(true);

    await userEvent.type(name, '-renamed');
    expect(apply.hasAttribute('disabled')).toBe(false);
  });

  it('locks the destructive and identity controls while the entry runs', async () => {
    showServer(true);
    const name = await screen.findByLabelText(
      m['entry.settings.server_name'](),
    );
    expect(name.hasAttribute('disabled')).toBe(true);
    expect(
      screen
        .getByRole('button', { name: m['entry.settings.change_version']() })
        .hasAttribute('disabled'),
    ).toBe(true);
    expect(
      screen
        .getByRole('button', { name: m['entry.settings.remove.server']() })
        .hasAttribute('disabled'),
    ).toBe(true);
  });

  it('offers the backup schedule on a server', async () => {
    showServer();
    expect(
      await screen.findByLabelText(m['entry.settings.keep_backups']()),
    ).toBeDefined();
  });

  it('has no backup schedule on an instance', async () => {
    showInstance();
    await screen.findByLabelText(m['entry.settings.instance_name']());
    expect(
      screen.queryByLabelText(m['entry.settings.keep_backups']()),
    ).toBeNull();
  });

  it('names removal after the server it would remove', async () => {
    showServer();
    expect(
      await screen.findByRole('button', {
        name: m['entry.settings.remove.server'](),
      }),
    ).toBeDefined();
  });

  it('names removal after the instance it would remove', async () => {
    showInstance();
    expect(
      await screen.findByRole('button', {
        name: m['entry.settings.remove.instance'](),
      }),
    ).toBeDefined();
  });

  it('writes the shared config, and the backup schedule with it', async () => {
    showServer();
    const args = await screen.findByLabelText(
      m['entry.settings.java_arguments'](),
    );
    await userEvent.clear(args);
    await userEvent.type(args, '-Xshare:off');

    const applies = screen.getAllByRole('button', {
      name: m['app.action.apply'](),
    });
    await userEvent.click(applies[applies.length - 1]);

    await waitFor(() =>
      expect(screen.getByText(m['app.toast.saved']())).toBeDefined(),
    );
  });

  it('opens the version dialog from the tab', async () => {
    showServer();
    await userEvent.click(
      await screen.findByRole('button', {
        name: m['entry.settings.change_version'](),
      }),
    );
    const dialog = await screen.findByRole('dialog');
    expect(dialog.textContent).toContain(m['entry.settings.allow_downgrade']());
  });
});
