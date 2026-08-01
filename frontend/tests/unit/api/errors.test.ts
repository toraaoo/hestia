import { describe, expect, it } from 'vitest';
import type { ErrorInfo } from '@/api';
import { errorMessage, errorMessageFromInfo, HestiaError } from '@/api';

const info = (value: Record<string, unknown>) => value as unknown as ErrorInfo;

describe('errorMessageFromInfo', () => {
  it('interpolates the variant’s own fields', () => {
    const rendered = errorMessageFromInfo(
      info({ kind: 'account_not_found', reference: 'steve' }),
    );
    expect(rendered).toBe("No account matches 'steve'.");
  });

  it('labels a token field through the shared vocabulary', () => {
    const rendered = errorMessageFromInfo(
      info({ kind: 'already_exists', entry: 'server', name: 'smp' }),
    );
    expect(rendered).toContain('smp');
    expect(rendered).not.toContain('{entry}');
  });

  it('joins a list field into one readable value', () => {
    const rendered = errorMessageFromInfo(
      info({ kind: 'archive_unsupported', format: 'mrpack', component: ['a', 'b'] }),
    );
    expect(rendered).toContain('a, b');
  });

  it('is empty for a kind the catalogue does not know', () => {
    expect(errorMessageFromInfo(info({ kind: 'not_a_real_kind' }))).toBe('');
  });
});

describe('errorMessage', () => {
  it('prefers the structured variant message', () => {
    const error = new HestiaError(
      'handler_error',
      'raw',
      info({ kind: 'account_not_found', reference: 'steve' }),
    );
    expect(errorMessage(error)).toBe("No account matches 'steve'.");
  });

  it('falls back to the coarse code when the variant is unknown', () => {
    const error = new HestiaError('not_found', 'raw', info({ kind: 'nope' }));
    expect(errorMessage(error)).not.toBe('raw');
    expect(errorMessage(error)).not.toBe('');
  });

  it('falls back to the code when there is no structured info at all', () => {
    expect(errorMessage(new HestiaError('timeout', 'raw'))).not.toBe('raw');
  });

  it('passes a plain Error through untouched', () => {
    expect(errorMessage(new Error('boom'))).toBe('boom');
  });

  it('stringifies anything else that was thrown', () => {
    expect(errorMessage('boom')).toBe('boom');
    expect(errorMessage(42)).toBe('42');
  });
});
