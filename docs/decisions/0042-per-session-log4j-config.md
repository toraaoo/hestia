# Per-session logs come from a generated Log4j2 config, not a captured pipe

*Applies to: [Servers & instances](../architecture/entries.md)*

Sessions share one `data/`, so they would all write `logs/latest.log`. Rather
than capture each session's stdout (a pipe the daemon owns, which dies on a
daemon restart and can't be re-established for an adopted process — the same
constraint that made the console RCON), each launch is pointed at its own
generated config via `-Dlog4j.configurationFile`, writing to
`<instance>/logs/session-<seq>.log`. That is a real file the game writes, so it
survives a daemon restart and the supervisor tails it by `LogSource::File`
exactly as before. The generated config is Log4Shell-safe — `%m{nolookups}` in
the pattern plus a belt-and-suspenders `-Dlog4j2.formatMsgNoLookups=true` — so
overriding Mojang's bundled config never re-opens CVE-2021-44228 on the older
versions Mojang had patched. The log lives under the instance root, not `data/`,
so it stays out of backups.
