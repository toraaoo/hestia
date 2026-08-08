# The node registry belongs to the front-ends, not to any daemon

*Applies to: [Front-ends](../architecture/frontends.md)*

Managing several machines needs something that holds the list of them. The
tempting place is the daemon — it is already resident, already owns documents in
the data home, already has a config vocabulary. That would be the wrong move: a
daemon storing other daemons' URLs and keys is a coordinator, and a coordinator
needs reconciliation, an availability story, and a second config surface that an
agent does not. It also makes one compromised launcher a pivot onto every
machine it knows, and it makes controlling a remote node depend on the *local*
daemon being up — a failure mode that buys nothing. `hestiad` stays ignorant of
every node but itself, which is the same property that makes the engine boundary
hold.

The registry lives in **`client`**, beside the HTTP transport, because both
front-ends are control planes. Putting it in the Tauri shell was the first
answer and it was wrong: `hestia --node prod` would have had no list, so a node
would be added twice and its key would live in two places. One module, two
callers, one list.

It splits three ways, and the split is the design:

- **The webview never holds a secret.** It receives node *metadata* — id, label,
  URL, last-seen — and picks an active node. Keys never cross into JavaScript, so
  an XSS anywhere in the content browser cannot exfiltrate node credentials.
- **Metadata and secret are stored apart.** `nodes.json` in the data home carries
  the list; the key goes to the OS keyring, falling back to an owner-only file
  where no keyring answers — a control plane is often a headless box, and
  refusing to hold a key there would mean the CLI could not drive a fleet at all.
  Either way a synced or backed-up `nodes.json` is worthless on its own.
- **The caller picks the transport.** `bridge.rs` keeps its single generic
  `ipc_call` command ([0049](0049-desktop-bridge-is-one-generic-command.md)) and
  the CLI keeps its `connect()`; both swap what is *underneath* — unix socket
  when the active node is local, HTTP when it is remote — and the HTTP envelope
  is unwrapped back into the shape each already consumes. `frontend/src/api/` and
  `frontend/src/queries/` do not change.

What this does not give is shared state: one operator, no second person seeing
the same fleet, no server-side audit trail, and no scheduled work against a
remote node while the control plane is closed. That is an accepted trade. If it
stops being one, the registry moves to `hestia-web/apps/api` — which already has
users, roles, `access_policy` rows and `hst_` keys — and nothing in the daemon
changes, because the daemon was never told other nodes exist.

Rejected: a `panel` mode on `hestiad`, or daemons connecting to each other, which
grows coordinator state and failure modes inside an agent and gives the launcher
two very different things to be. Rejected too: the node list in the webview,
which puts long-lived credentials one scripting bug away from leaving the
machine; and the registry in the desktop shell alone, which leaves the CLI — an
equal front-end — unable to reach the fleet.
