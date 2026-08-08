# HTTP is a second door onto the same router, and carries its own envelope

*Applies to: [The socket boundary](../architecture/wire.md)*

Remote server management needs a transport a browser, a script and a reverse
proxy all understand. The socket envelope is not that: `{v, ok, payload|error,
id}` exists to carry a correlation id and a protocol major over a
length-prefixed byte stream, and HTTP already has a status line and headers for
both. Tunnelling one inside the other would mean every response is 200 and every
failure is invisible to anything that reads HTTP — caches, proxies, monitoring,
and `curl`, which is a first-class client here.

So the HTTP surface carries its own envelope, and it is the one
`hestia-web/apps/api` already defines
(`packages/contract/src/core/envelope.ts`): `success`, `message`, `data`,
`code`. Two services, one response shape, and `packages/sdk` can reach a daemon
without a second client.

The split inside a failure is the part worth stating. `code` is the closed
nine-value set (`NOT_FOUND`, `VALIDATION_FAILED`, …) that a generic integrator
switches on. `data` is the full serialized `ErrorInfo`, tagged on `kind`, which a
Hestia-aware client localizes through the same `error.*` message tables it uses
over the socket. One vocabulary at two granularities; nothing new invented, and
no third naming seam that can drift. `message` is developer-facing English from
`ErrorInfo`'s `Display` — for logs and `curl`, never for UI.

Both doors dispatch into `Router::route`, which takes a decoded `Request` and
returns a `Response` and does not know a socket exists. Handlers, the account
gate, the tracing span and the timing log are all reached unchanged, so
behaviour has exactly one definition. `ipc` gains nothing and learns nothing: it
stays the socket's transport and stays domain-free.

Rejected: the socket envelope over HTTP, for the reasons above. Rejected too:
one `tokio-listener` serving both transports through a single router — it is the
obvious way to share code, but the two envelopes are the point, and a shared
router makes it an accident waiting to happen rather than a boundary.
