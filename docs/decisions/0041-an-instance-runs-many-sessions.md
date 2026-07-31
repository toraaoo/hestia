# An instance runs many sessions; a server runs one

*Applies to: [Servers & instances](../architecture/entries.md)*

A client can be launched more than once at a time — **opt-in twice**. The
capability is off launcher-wide until `instance.multi-session` is turned on:
concurrent sessions share one `data/` and Minecraft arbitrates only a world, so
the rest (configs, the content mirror, the options file) is last-writer-wins and
that is not a default anyone should discover by accident. With it on, `launch`
*still* refuses a running instance unless the `new_session` param
(`--new-session`) asks for a concurrent one, so the common case stays a single
session. The two refusals are distinct — one is fixed by stopping the instance,
the other by a setting — rather than one message that guesses which the caller
meant. Under the hood `instance-<id>` is no
longer a single supervisor key — it splits into an *entry key* (`instance-<id>`,
still the unit for the backup/update/content/rename guards and their in-flight
sets) and a per-launch *session key* (`instance-<id>_<seq>`). Ids are `[0-9a-f]`
(a uuid hex string), never `_`, so a session prefix `instance-<id>_` can't
collide across instances; every former singular lookup (status, stop, logs,
running-check) becomes a prefix query over the supervisor's flat table, so the
supervisor and its on-disk records need no change — each session just gets a
distinct id. `stop` fans out to every session (or a named one); `logs` targets
the newest running session (or a named one). Servers stay singular
(`server-<id>`): a world has one authoritative writer. Two sessions of one
instance share its single `data/` — Minecraft's own `session.lock` arbitrates a
world, and each session gets a [private log](0042-per-session-log4j-config.md)
so their output never interleaves.
