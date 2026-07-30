# The envelope seam fails closed, so no decode site can forget the version check

*Applies to: [The socket boundary](../architecture/wire.md)*

`PROTOCOL_VERSION` is `1`, same-major only — but the rule is only real if every
decode enforces it. `compatible()` began as a free helper with *zero* call
sites: `decode_request` handed the version out as a plain field each caller was
free to ignore, and defaulted a **missing** `v` to the current version, so any
envelope (`v: 0`, `v: 99`, absent) decoded into a valid request and dispatched —
the seam failed *open*, and the "same-major only" rule was unenforced anywhere
in the tree. Now the type carries the invariant:
`decode_request`/`decode_response` return a typed `DecodeError` (`Malformed` vs
`IncompatibleVersion { got, want }`) and **refuse to construct a frame at all**
for a foreign major or a missing `v` (a missing version is malformed, never an
implicit "current"). The daemon maps that error to a `version_mismatch` response
(`ErrorInfo::IncompatibleVersion`) and the client tears the connection down
rather than silently consuming a foreign-major daemon — a junk frame is still
ignored, but a version mismatch is refused. Both directions are pinned in
`protocol.rs`'s unit tests. The rejected alternative was an `if !compatible(v)`
guard inside the daemon's serve loop: a band-aid that leaves the same hole open
at every other decode site and keeps the check opt-in for the next person who
decodes a frame.
