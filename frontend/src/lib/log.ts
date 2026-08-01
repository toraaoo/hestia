import { invoke } from '@tauri-apps/api/core';
import pino from 'pino';

const FORWARD_FROM = 'warn';

const dev = import.meta.env.DEV;
const testing = import.meta.env.MODE === 'test';

function forward(level: string, logEvent: pino.LogEvent) {
  const [first, ...rest] = logEvent.messages;
  const message = typeof first === 'string' ? first : JSON.stringify(first);
  const fields = [...logEvent.bindings, ...rest];
  invoke('log_write', {
    level,
    message,
    fields: fields.length > 0 ? JSON.stringify(fields) : null,
  }).catch(() => {});
}

export const log = pino({
  level: testing ? 'silent' : dev ? 'debug' : FORWARD_FROM,
  browser: {
    asObject: false,
    transmit: { level: FORWARD_FROM, send: forward },
  },
});

export function logger(feature: string) {
  return log.child({ feature });
}
