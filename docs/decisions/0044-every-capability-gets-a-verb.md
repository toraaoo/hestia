# Every daemon capability gets a scriptable verb, or a written reason it has none

*Applies to: [Front-ends](../architecture/frontends.md)*

Diffing the channels registered in `daemon/src/services/` against the CLI
grammar found four families with no verb. Three are deliberate and documented —
`skin.*`/`cape.*` (picking a skin is visual), `profile.*` and
`instance.profile.*` (desktop surfaces by design). Two were drift:
`instance.worlds`, reachable only as a side effect of the datapack picker, and
the whole `process.*` surface, which nothing but `daemon stop`'s internal
workload check ever read. Both now have verbs. The distinction that matters is
*stated intent*: a channel with no CLI verb is fine when the architecture says
why, and a bug when it does not — so the audit is repeatable rather than a
matter of taste. `hestia process` is deliberately the supervisor's own view,
keyed by supervisor id, not a second way to drive an entry: it answers the
questions the entry-scoped verbs structurally cannot (every workload at once,
and a process whose entry was removed under it).
