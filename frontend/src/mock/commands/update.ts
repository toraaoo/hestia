/** `changelog` — the notes this build compiles in. */
import type { Handlers } from '../support';

const NOTES = `## 0.0.1-mock

- Served by the browser fixture daemon.
- Every page is driven by \`src/mock\`, not by \`hestiad\`.
`;

export const commands: Handlers = {
  changelog: () => NOTES,
};
