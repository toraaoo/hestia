# Reachability is observed from real traffic, and offline is a state the whole system reads

*Applies to: [The engine](../architecture/engine.md), [Front-ends](../architecture/frontends.md)*

The launcher assumed it was online. No outbound request had a timeout — not the
downloader, not the metadata clients, not Microsoft sign-in — so a dropped
connection did not fail anything: it *stopped* it. A connection that goes away
usually stops delivering rather than resetting, so a materialize would sit at
whatever byte it reached until the user cancelled the job. There were also three
separate `reqwest` clients, two of them rebuilt per request, which is why the
one with the pooling comment on it was not the one Microsoft auth used.

**Every outbound request now crosses one seam, `engine::net`**, which owns four
things that used to be nobody's: the pooled client and its timeouts (10s
connect, 30s between bytes, 45s total for a metadata call and none for a
download — a large artifact on a slow line is not a failure), a bounded retry,
one classification of what went wrong, and the reachability state.

## The state is derived from the requests, not from a separate check

`net::Network` is `Online | Offline | Unknown`, and it moves because a real
request moved it: any answer marks the network up — including a 404, since the
service plainly answered — and only a transport failure with its retries spent
marks it down. A single blip therefore never flips every front-end to offline
and back.

The alternative was a connectivity poller, and it was rejected because it can
disagree with the thing that matters. A poller says "online" while the one host
a launch is about to need is unreachable, and the user gets a green light
followed by a failure. Traffic cannot lie about itself.

Observation alone leaves two gaps, and a probe fills exactly those: nothing has
been attempted yet (`Unknown` at startup, which is why there are three states
and not two — the launcher has no grounds to claim either), and nothing is being
attempted now, so a network that came back would go unnoticed until the user
clicked something. The daemon ticks `Network::refresh`, which probes only when
the state has gone stale — every 10s while offline, and while online only once
traffic has been quiet for a minute. It probes Mojang's version manifest rather
than a third-party connectivity endpoint: it is a service the launcher already
depends on, so the probe neither tells anyone new that hestia is running nor
reports online when the host every launch needs is down.

## Offline is a typed error, and pinned offline is a different one

`ErrorInfo::Offline { service }` means the request never left the machine —
distinct from `Upstream`, where the service answered and the answer was a
failure. Only the first means "try again when you are back online", so it gets
its own `ipc::errors` code and front-ends branch on it once.

`network.offline` pins the launcher offline, and raises `OfflineMode` rather
than a flag on `Offline`. Two variants rather than one with a boolean because
the remedies differ — wait for a connection, or turn a setting off — and a
front-end renders one localized string per variant with no branching of its own.

## What a cached catalogue buys, and what it does not

A version list changes rarely, and a stale one is far more useful than a failed
page, so a catalogue read falls back to the last good copy under `meta/`. That
is what makes creating an instance work on a version you have picked before.

It is deliberately not marked stale on the wire. Doing that would mean a warning
threaded through every provider signature; the front-ends already know the
network state, so a cached answer is never mistaken for a live one — the version
picker says so itself. Only catalogues are cached. Content search is not: the
result space is unbounded and caching it is speculative.

## A launch does not need the network

Everything a launch needs is on disk after the first one — `materialize` skips
what is present, and most flavors install nothing — so the only thing standing
between a materialized instance and a dropped connection was token rotation.
Access tokens last about a day, so "no internet since yesterday" meant no
singleplayer either.

A rotation that cannot reach Microsoft now answers with the stored token marked
stale, and the session carries a `SessionNotVerified` warning. A refresh token
Microsoft actively *rejects* is still fatal: that is an expired sign-in, not a
network problem. `--offline` asks for the same thing deliberately and never
contacts Microsoft at all, borrowing the account's name and uuid so worlds and
per-player data still match. Both run on a token the game cannot authenticate
with, which is the point — singleplayer works and multiplayer is refused, which
is what an unverified session is entitled to.

## The front-ends say it once, and do not block

A lost *daemon* covers the window ([0053](0053-offline-is-one-state.md)) because
nothing works without it. A lost *connection* must not: instances still launch
and servers still run. So the network indicator sits beside the daemon light
rather than replacing it, and each surface that genuinely needs upstream — the
content browser, the install picker, the skin library, Java installs — renders
its own offline state instead of an error toast. The `offline` code is silenced
in the toast path for the same reason a lost socket is: it is a state the status
bar reports, not an error per call.
