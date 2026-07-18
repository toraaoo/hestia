/** The `config.*` channels. `home` and `autostart` are the reserved keys. */
import { call, tryCall } from './core/ipc';

export async function get(key: string): Promise<unknown | null> {
  const result = await tryCall<{ value: unknown }>(
    'config.get',
    { key },
    { raw: true },
  );
  return result?.value ?? null;
}

export async function set(key: string, value: unknown): Promise<void> {
  await call('config.set', { key, value }, { raw: true });
}

export async function list(): Promise<Record<string, unknown>> {
  const result = await call<{ entries: Record<string, unknown> }>(
    'config.list',
    {},
    { raw: true },
  );
  return result.entries;
}
