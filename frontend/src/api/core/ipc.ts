/**
 * The one seam every daemon call crosses: the shell's generic `ipc_call`
 * command forwards `{ channel, payload }` over the local socket and answers
 * with the response payload or a `{ code, message }` rejection.
 *
 * The wire is snake_case (it mirrors `crates/proto`); the TS side is camelCase.
 * This seam is where the two meet: every outbound payload is decamelized to the
 * wire shape and every response is camelized back, so the rest of the frontend
 * only ever sees camelCase and the type mirrors read like idiomatic TS. Pass
 * `raw` for the schema-less channels whose keys are user data, not struct
 * fields (`config.*`), where blind key conversion would corrupt them.
 */
import { invoke } from '@tauri-apps/api/core';
import { camelizeKeys, decamelizeKeys } from 'humps';

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
  /**
   * Skip snake_case⇄camelCase key conversion in both directions. For channels
   * whose payload keys are user data rather than struct fields (`config.*`),
   * where converting them would rewrite the keys themselves.
   */
  raw?: boolean;
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
  const payload = params ?? {};
  try {
    const result = await invoke<unknown>('ipc_call', {
      channel,
      payload: options.raw ? payload : decamelizeKeys(payload),
      timeoutMs: options.timeoutMs,
    });
    return (options.raw ? result : camelizeKeys(result)) as T;
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
