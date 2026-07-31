/**
 * `update_*` and `changelog` — self-update, which the shell drives through
 * `tauri-plugin-updater` rather than the daemon. Nothing here can replace a
 * running binary, so the fixture reports "up to date" and serves the notes
 * this build compiles in.
 */
import type { Handlers } from '../support';

const NOTES = `## 0.0.1-mock

- Served by the browser fixture daemon.
- Every page is driven by \`src/mock\`, not by \`hestiad\`.
`;

export const commands: Handlers = {
  update_check: () => null,
  update_install: () => null,
  changelog: () => NOTES,
};
