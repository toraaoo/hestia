# The remote surface is a positive allowlist, and a scope is checked against the route

*Applies to: [The daemon](../architecture/daemon.md)*

A denylist widens itself. If the HTTP layer exposed every channel except a list of
excluded ones, each new `handle::<C>` in `services/` would be remotely reachable
the moment it was written, and staying safe would depend on whoever added it
remembering a file they had no reason to open. The exclusion has to be structural,
not a checklist — the same reasoning that put `requires_account` in the router
rather than in each instance handler
([0033](0033-instance-surface-gated-on-an-account.md)).

So the HTTP router mounts an explicit list of server-side contracts. A channel not
on that list has no path at all: the answer is 404 on an unmounted route, not 403
on a mounted one. `account.*`, `skin.*`, `instance.*`, `sync.*`, `update.*`,
`config.set home` and the key-management channels are unreachable rather than
refused, and adding a domain to `services/` never silently widens what a remote
caller can do.

Each mounted route also declares the scope it costs — `server:read`,
`server:control`, `server:write`, `server:backup`, `server:create`,
`server:delete` — and a key declares what it holds, optionally narrowed to a set of
server ids. The check is per route, against the key, never inferred from the key
merely being valid. That distinction is not theoretical: Pterodactyl's
CVE-2026-54593 was Wings accepting any correctly-signed token carrying the right
ids regardless of what the token had been issued *for*, which let a low-privilege
operation's token perform a high-privilege one.

Rejected: a prefix rule — "anything under `server.`". It reads as equivalent and is
not. `server.content.add` makes the daemon fetch an arbitrary URL and write to
disk, which is a different risk from `server.list` and deserves a scope of its own
rather than inheriting one from its name. Rejected too: deferring scopes until
mutation ships, which would mean retrofitting authorization onto routes already in
use — the exact shape of the mistake above.
