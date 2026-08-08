# The HTTP surface is versioned in the path, independently of the wire protocol

*Applies to: [The socket boundary](../architecture/wire.md)*

There are now two contracts, and they change for different reasons on different
clocks. `PROTOCOL_VERSION` describes a frame: it is negotiated once at connect,
it fails closed on a foreign major ([0001](0001-envelope-fails-closed.md)), and
exactly one version is ever live. The HTTP resource contract is discovered from a
URL, and its whole job is to serve two majors at once while callers migrate.

Tying them would force a socket bump for an HTTP-only change — which every local
front-end would then have to survive for no reason — and would make "serve v1 and
v2 together" impossible, since a connection speaks one major or none.

So `/api/v1` is a mount, versioned in the path. It is visible in logs, it works
from `curl`, and a future `/api/v2` is mounted beside it with `Deprecation` and
`Sunset` headers on the old one for the announced window. Within a major the rule
is additive only: new optional fields and new routes, never a removed or retyped
field, and never a changed status or `code` for a condition that already has one.
The `#[serde(default)]` discipline every proto payload already carries gives that
tolerance for free in both directions.

`GET /api/versions` is deliberately outside both: it reports the HTTP majors
served, the daemon version, and the protocol major, so a client that guessed
wrong learns what to ask for instead. Every response also carries
`X-Hestia-Api-Version` and `X-Hestia-Daemon`.

Neither line derives from `common::app::VERSION`. A release that changes no
contract bumps neither.

Rejected: `Accept`-header negotiation. It keeps one URL per resource, but the
version becomes a property of a request rather than of a mount — invisible in
logs, awkward from a shell, and impossible to diff by reading the route table.
Rejected too: versioning by daemon release, which would break a client every time
an unrelated feature shipped.
