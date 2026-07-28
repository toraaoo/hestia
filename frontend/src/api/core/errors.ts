/**
 * Localizes a daemon error. The structured `ErrorInfo` is rendered
 * **generically**: `error.kind.<kind>` is the message template and the variant's
 * fields are its interpolation params, with token-enum fields (server, memory,
 * adoptium, …) resolved through the shared `error.token.*` table. A coarse
 * `error.code.*` message is the fallback, then the raw message.
 *
 * There is deliberately no per-variant `switch`: adding an `ErrorInfo` variant
 * needs only message keys, never a change here. The only per-field knowledge is
 * `TOKEN_FIELDS` — the fields whose value is itself an enum to be labelled.
 */
import { m } from '@/paraglide/messages.js';
import type { ErrorInfo } from '../types/error';
import { HestiaError } from './ipc';

// paraglide's `m` is a flat object of message functions; error keys are looked
// up dynamically, so it is accessed untyped here.
const msg = m as unknown as Record<
  string,
  (params?: Record<string, unknown>) => string
>;

// Fields whose value is a token enum resolved through `error.token.*`; every
// other field interpolates raw. A value with no token entry falls back to
// itself, so listing a free-text field here would be harmless.
const TOKEN_FIELDS = new Set([
  'field',
  'reason',
  'entry',
  'scope',
  'service',
  'source',
  'operation',
  'what',
  'actual',
  'expected',
  'requested',
]);

function token(value: unknown): string {
  return typeof value === 'string'
    ? (msg[`error.token.${value}`]?.() ?? value)
    : String(value);
}

/** The localized message for a structured daemon error. */
export function errorMessageFromInfo(info: ErrorInfo): string {
  const params: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(info)) {
    if (key === 'kind') continue;
    if (Array.isArray(value)) params[key] = value.map(token).join(', ');
    else if (TOKEN_FIELDS.has(key)) params[key] = token(value);
    else params[key] = value;
  }
  return msg[`error.kind.${info.kind}`]?.(params) ?? '';
}

function codeFallback(code: string): string {
  return msg[`error.code.${code}`]?.() ?? '';
}

/** The localized, user-facing string for any daemon or transport error. */
export function errorMessage(error: unknown): string {
  if (!(error instanceof HestiaError)) {
    return error instanceof Error ? error.message : String(error);
  }
  if (error.info) {
    const rendered = errorMessageFromInfo(error.info);
    if (rendered) return rendered;
  }
  return codeFallback(error.code) || error.message;
}
