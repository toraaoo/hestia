/**
 * Localizes a daemon error: renders the structured `ErrorInfo` through the
 * `error.kind.*` messages (resolving token fields to their own labels), falling
 * back to a generic message keyed by the coarse `code`, then the raw message.
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

function label(category: string, value: string): string {
  return msg[`error.${category}.${value}`]?.() ?? value;
}

function kind(name: string, params: Record<string, unknown> = {}): string {
  return msg[`error.kind.${name}`]?.(params) ?? '';
}

function codeFallback(code: string): string {
  return msg[`error.code.${code}`]?.() ?? '';
}

function fromInfo(info: ErrorInfo): string {
  switch (info.kind) {
    case 'field_required':
      return kind(info.kind, { field: label('field', info.field) });
    case 'fields_required':
      return kind(info.kind, {
        fields: info.fields.map((f) => label('field', f)).join(', '),
      });
    case 'invalid_value':
      return kind(info.kind, {
        field: label('field', info.field),
        reason: label('reason', info.reason),
      });
    case 'mutually_exclusive':
      return kind(info.kind, { options: info.options.join(', ') });
    case 'nothing_to_do':
      return kind(info.kind, { what: label('task', info.what) });
    case 'unsupported_operation':
      return kind(info.kind, { reason: label('unsupported', info.reason) });
    case 'entry_not_found':
      return kind(info.kind, {
        entry: label('entry', info.entry),
        reference: info.reference,
      });
    case 'entry_running':
    case 'not_running':
      return kind(info.kind, {
        entry: label('entry', info.entry),
        name: info.name,
      });
    case 'already_exists':
      return kind(info.kind, {
        entry: label('nameable', info.entry),
        name: info.name,
      });
    case 'profile_not_found':
      return kind(info.kind, {
        scope: label('scope', info.scope),
        name: info.name,
      });
    case 'sync_target_invalid':
      return kind(info.kind, {
        path: info.path,
        reason: label('sync_reason', info.reason),
      });
    case 'upstream':
      return kind(info.kind, {
        service: label('service', info.service),
        detail: info.detail,
      });
    case 'io':
      return kind(info.kind, {
        operation: label('io_op', info.operation),
        detail: info.detail,
      });
    case 'eula_required':
    case 'sign_in_required':
    case 'login_declined':
    case 'login_timed_out':
      return kind(info.kind);
    default:
      return kind(info.kind, info as unknown as Record<string, unknown>);
  }
}

/** The localized, user-facing string for any daemon or transport error. */
export function errorMessage(error: unknown): string {
  if (!(error instanceof HestiaError)) {
    return error instanceof Error ? error.message : String(error);
  }
  if (error.info) {
    const rendered = fromInfo(error.info);
    if (rendered) return rendered;
  }
  return codeFallback(error.code) || error.message;
}
