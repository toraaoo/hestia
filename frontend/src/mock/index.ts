// Dev-only fixture bridge: fakes `window.__TAURI_INTERNALS__` so the frontend
// runs in a plain browser (no daemon, no Tauri shell). See ./fixtures.
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { channels, commands } from './fixtures';

// Safety net for an unlisted channel: an empty array-backed proxy whose every
// property is another such proxy. It is `Array.isArray`-true (a valid, empty
// React child and mappable) and never yields `undefined`, so an unmocked read
// degrades to "empty" instead of crashing a query or a `.length`/`.x` access.
function emptyProxy(): unknown {
  return new Proxy([] as unknown[], {
    get(target, prop, receiver) {
      if (prop in target || typeof prop === 'symbol') {
        return Reflect.get(target, prop, receiver);
      }
      return emptyProxy();
    },
  });
}

function dispatch(cmd: string, args: Record<string, unknown>): unknown {
  if (cmd === 'ipc_call') {
    const channel = String(args.channel);
    const payload = (args.payload ?? {}) as Record<string, unknown>;
    const handler = channels[channel];
    if (handler) return handler(payload);
    console.warn(
      `[mock] no fixture for channel "${channel}" — returning empty`,
    );
    return emptyProxy();
  }
  const command = commands[cmd];
  if (command) return command(args);
  console.warn(`[mock] no fixture for command "${cmd}" — returning null`);
  return null;
}

export async function installBrowserMock(): Promise<void> {
  mockWindows('main');
  mockIPC(
    (cmd, args) => dispatch(cmd, (args ?? {}) as Record<string, unknown>),
    {
      shouldMockEvents: true,
    },
  );
  console.info('[mock] running against fixture daemon (browser dev mode)');
}
