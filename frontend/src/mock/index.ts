/**
 * The browser fixture daemon.
 *
 * Dev-only: it fakes `window.__TAURI_INTERNALS__` so the frontend runs in a
 * plain browser with no daemon and no Tauri shell behind it. Stripped from the
 * desktop build, and skipped when the real shell is present.
 *
 * It is laid out the way the thing it replaces is:
 *
 * | Here            | Stands in for                          |
 * |-----------------|----------------------------------------|
 * | `state/`        | `crates/engine` — the mutable world     |
 * | `channels/`     | `crates/daemon/src/services/` — one module per domain |
 * | `commands/`     | `crates/desktop/src/commands/` + the bundled plugins |
 * | `router.ts`     | the daemon's router and the shell's bridge |
 * | `job.ts`/`bus.ts` | the job managers and the event hub    |
 *
 * Adding a channel is one line in its domain's map; adding a domain is a
 * module plus a line in `channels/index.ts`. Fixtures are typed against the
 * generated `proto` mirrors, so a wire change breaks the typecheck here rather
 * than silently serving a stale shape.
 */
import { installInternals } from './internals';

export async function installBrowserMock(): Promise<void> {
  installInternals();
  console.info('[mock] running against the fixture daemon (browser dev mode)');
}
