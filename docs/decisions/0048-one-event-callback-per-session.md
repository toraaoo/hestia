# One event-callback slot per client `Session`

*Applies to: [The socket boundary](../architecture/wire.md)*

`run_job` and `subscribe` both claim the session's single event callback, so a
session driver must serialize event-driven calls: plain request/response calls
(search, detail, versions) may interleave freely, and the one job
(`content.add`, the create, a log subscription) runs by itself. The content
session and the wizards follow this rule; violating it silently drops events.
