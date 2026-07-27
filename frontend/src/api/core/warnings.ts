/**
 * Localizes a daemon warning — a degraded outcome of an operation that
 * nonetheless succeeded. Rendered **generically**, exactly like `ErrorInfo`:
 * `warning.kind.<kind>` is the headline template, `warning.hint.<kind>` the
 * remediation, and the variant's fields are the interpolation params, with
 * token-enum fields resolved through the shared `warning.token.*` table.
 *
 * There is deliberately no per-variant `switch`: adding a `WarningInfo` variant
 * needs only message keys, never a change here.
 */
import { m } from '@/paraglide/messages.js';
import type { WarningInfo } from '../types/warning';

const msg = m as unknown as Record<
  string,
  (params?: Record<string, unknown>) => string
>;

// Fields whose value is itself an enum, labelled through `warning.token.*`.
const TOKEN_FIELDS = new Set(['reason']);

function token(value: unknown): string {
  return typeof value === 'string'
    ? (msg[`warning.token.${value}`]?.() ?? value)
    : String(value);
}

function params(info: WarningInfo): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(info)) {
    if (key === 'kind') continue;
    out[key] = TOKEN_FIELDS.has(key) ? token(value) : value;
  }
  return out;
}

/** The localized headline: what is degraded. */
export function warningMessage(info: WarningInfo): string {
  return msg[`warning.kind.${info.kind}`]?.(params(info)) ?? '';
}

/** The localized hint: what the user can do about it. */
export function warningHint(info: WarningInfo): string {
  return msg[`warning.hint.${info.kind}`]?.(params(info)) ?? '';
}
