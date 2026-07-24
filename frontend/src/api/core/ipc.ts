/**
 * The one seam every daemon call crosses: the shell's generic `ipc_call`
 * command forwards `{ channel, payload }` over the local socket and answers
 * with the response payload or a `{ code, message }` rejection.
 *
 * The wire is camelCase — `crates/proto` carries `rename_all = "camelCase"`, so
 * the daemon speaks the same shape the type mirrors describe and no key
 * conversion happens here. (A dynamic-key payload like `config.*`'s opaque
 * value therefore also passes through untouched.)
 */
import { invoke } from '@tauri-apps/api/core';
import type { ErrorInfo } from '../types/error';

/** Default per-call timeout, matching the client SDK's `CALL_TIMEOUT`. */
export const CALL_TIMEOUT_MS = 10_000;

/** Error codes raised by the daemon (`ipc::errors`). */
export const NOT_FOUND = 'not_found';
export const BAD_REQUEST = 'bad_request';
export const HANDLER_ERROR = 'handler_error';
export const UNKNOWN_CHANNEL = 'unknown_channel';
export const UNAUTHORIZED = 'unauthorized';
/** Error codes raised by the shell's bridge for transport failures. */
export const TIMEOUT = 'timeout';
export const CONNECTION_LOST = 'connection_lost';
export const TRANSPORT = 'transport';

export class HestiaError extends Error {
  readonly code: string;
  /** The structured daemon error (`proto::error::ErrorInfo`) to localize from. */
  readonly info: ErrorInfo | null;

  constructor(code: string, message: string, info: ErrorInfo | null = null) {
    super(message);
    this.name = 'HestiaError';
    this.code = code;
    this.info = info;
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
    const { code, message, info } = raw as {
      code: string;
      message: string;
      info?: ErrorInfo | null;
    };
    return new HestiaError(code, message, info ?? null);
  }
  return new HestiaError(TRANSPORT, String(raw));
}

export async function call<T>(
  channel: string,
  params: unknown = {},
  options: CallOptions = {},
): Promise<T> {
  try {
    const result = await invoke<unknown>('ipc_call', {
      channel,
      payload: params ?? {},
      timeoutMs: options.timeoutMs,
    });
    return result as T;
  } catch (raw) {
    throw toHestiaError(raw);
  }
}

/**
 * Invoke a bespoke shell command (not the generic `ipc_call` bridge). Reserved
 * for the few flows the frontend cannot drive over the socket alone — sign-in,
 * which opens a native webview and reads its redirect. Rejections share
 * `ipc_call`'s `{ code, message }` shape.
 */
export async function invokeCommand<T>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  try {
    return await invoke<T>(command, args);
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
