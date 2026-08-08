# The node registry belongs to the desktop shell; no daemon knows about another

*Applies to: [Front-ends](../architecture/frontends.md)*

Managing several machines needs something that holds the list of them. The
tempting place is the daemon — it is already resident, already owns documents in
the data home, already has a config vocabulary. That would be the wrong move: a
daemon storing other daemons' URLs and keys is a coordinator, and a coordinator
needs reconciliation, an availability story, and a second config surface that an
agent does not. `hestiad` stays ignorant of every node but itself, which is the
same property that makes the engine boundary hold.

The registry lives in the Tauri shell instead — the one component that is already
a client of many things and a server to none. It splits three ways, and the split
is the design:

- **The webview never holds a secret.** It receives node *metadata* — id, label,
  URL, last-seen — and picks an active node. Keys never cross into JavaScript, so
  an XSS anywhere in the content browser cannot exfiltrate node credentials.
- **The shell holds the registry.** `nodes.json` beside the desktop's config
  carries metadata; the keys go to the OS keyring. Metadata and secret are
  separated so a copied or backed-up config file is worthless on its own.
- **`bridge.rs` picks the transport.** It keeps its single generic `ipc_call`
  command ([0049](0049-desktop-bridge-is-one-generic-command.md)) and swaps what
  is underneath — unix socket when the active node is local, HTTP when it is
  remote — unwrapping the HTTP envelope back into the shape the frontend already
  consumes. `frontend/src/api/` and `frontend/src/queries/` do not change.

What this does not give is shared state: one operator, one machine, no second
person seeing the same fleet, no server-side audit trail, and a lost desktop
config loses the node list. That is an accepted trade for now. If it stops being
one, the registry moves to `hestia-web/apps/api` — which already has users,
roles, `access_policy` rows and `hst_` keys — and nothing in the daemon changes,
because the daemon was never told other nodes exist.

Rejected: a `panel` mode on `hestiad`, which grows coordinator state and failure
modes inside an agent and gives the launcher two very different things to be.
Rejected too: the node list in the webview, which puts long-lived credentials one
scripting bug away from leaving the machine.
