/**
 * The one seam every daemon call crosses: the shell's generic `ipc_call`
 * command forwards `{ channel, payload }` over the local socket and answers
 * with the response payload or a `{ code, message }` rejection.
 *
 * Channel names and payload shapes mirror `crates/proto` — wire fields stay
 * snake_case so the TS types can be audited against the Rust structs
 * one-to-one, with no mapping layer to drift.
 */
import { invoke } from '@tauri-apps/api/core';

/** Default per-call timeout, matching the client SDK's `CALL_TIMEOUT`. */
export const CALL_TIMEOUT_MS = 10_000;

/** Error codes raised by the daemon (`ipc::errors`). */
export const NOT_FOUND = 'not_found';
export const BAD_REQUEST = 'bad_request';
export const HANDLER_ERROR = 'handler_error';
export const UNKNOWN_CHANNEL = 'unknown_channel';
/** Error codes raised by the shell's bridge for transport failures. */
export const TIMEOUT = 'timeout';
export const CONNECTION_LOST = 'connection_lost';
export const TRANSPORT = 'transport';

export class HestiaError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'HestiaError';
    this.code = code;
  }
}

export function isNotFound(error: unknown): boolean {
  return error instanceof HestiaError && error.code === NOT_FOUND;
}

export interface CallOptions {
  timeoutMs?: number;
}

function toHestiaError(raw: unknown): HestiaError {
  if (raw && typeof raw === 'object' && 'code' in raw && 'message' in raw) {
    const { code, message } = raw as { code: string; message: string };
    return new HestiaError(code, message);
  }
  return new HestiaError(TRANSPORT, String(raw));
}

export async function call<T>(
  channel: string,
  params: unknown = {},
  options: CallOptions = {},
): Promise<T> {
  try {
    return await invoke<T>('ipc_call', {
      channel,
      payload: params ?? {},
      timeoutMs: options.timeoutMs,
    });
  } catch (raw) {
    throw toHestiaError(raw);
  }
}

/** Like `call`, but a `not_found` answer becomes `null` instead of throwing. */
export async function tryCall<T>(
  channel: string,
  params: unknown = {},
  options: CallOptions = {},
): Promise<T | null> {
  try {
    return await call<T>(channel, params, options);
  } catch (error) {
    if (isNotFound(error)) return null;
    throw error;
  }
}
