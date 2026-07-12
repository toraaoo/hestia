import { notFound } from "@tanstack/react-router";

/** Unwrap an optional lookup, surfacing the router's not-found state when absent. */
export function orNotFound<T>(value: T | undefined | null): T {
  // eslint-disable-next-line @typescript-eslint/only-throw-error -- the router catches its own non-Error marker
  if (value == null) throw notFound();
  return value;
}
