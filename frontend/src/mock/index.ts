// Dev-only fixture bridge: fakes `window.__TAURI_INTERNALS__` so the frontend
// runs in a plain browser (no daemon, no Tauri shell). See ./fixtures.
import { mockIPC } from '@tauri-apps/api/mocks';
import { channels, commands } from './fixtures';

function dispatch(cmd: string, args: Record<string, unknown>): unknown {
  if (cmd === 'ipc_call') {
    const channel = String(args.channel);
    const payload = (args.payload ?? {}) as Record<string, unknown>;
    const handler = channels[channel];
    if (handler) return handler(payload);
    console.warn(`[mock] no fixture for channel "${channel}" — returning {}`);
    return {};
  }
  const command = commands[cmd];
  if (command) return command(args);
  console.warn(`[mock] no fixture for command "${cmd}" — returning null`);
  return null;
}

export async function installBrowserMock(): Promise<void> {
  mockIPC(
    (cmd, args) => dispatch(cmd, (args ?? {}) as Record<string, unknown>),
    {
      shouldMockEvents: true,
    },
  );
  console.info('[mock] running against fixture daemon (browser dev mode)');
}
