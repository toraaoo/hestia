# A warning the user did not cause is a bug in the launcher, not a notice

*Applies to: [The socket boundary](../architecture/wire.md)*

The [structured-warning rule](0029-degraded-outcomes-ride-on-the-result.md)
earned its keep and then over-fired: the two warnings a normal user actually met
were both about hestia's own limitations. Every NeoForge create said its
property schema could not be derived (structural — see the NeoForge note), and
an instance whose `data/config` had contents — which a modpack's `overrides/`
puts there before the first launch ever runs — said `config` was not shared,
pointing at an `adopt` chore hestia could perfectly well do itself. Neither
followed from anything the user did, and the second's "remediation" was work the
daemon was declining to do.

So the fix in both cases was to **remove the degradation**, not to soften the
text. The schema run stopped using an argument file it could not resolve; the
folder guard was narrowed from "never touch a non-empty directory" to "never
overwrite" — a folder holding only the instance's own files is adopted at the
launch that would have warned, silently, because moving files into the store is
exactly what making it a target asked for and nothing can be lost. What survives
is a warning about a **name clash** (`NotSharedReason::Collides`), which the
user must resolve because either copy could be the one they want, and a foreign
link, which is theirs to repoint.

Two rules keep the automatic pass honest. **A modpack owns its config tree**: a
pack ships `config/` as part of what it is, so folding it into the store every
other instance reads would push one pack's settings onto all of them —
`Settings::Local` leaves those folders alone, with no warning, since it is a
deliberate outcome rather than a degraded one. It is the automatic pass only: an
`adopt` the user asks for still opts the folder in, and the link it leaves is
reconciled from then on, so a pack *can* share if that is what the user wants.
And **hestia never breaks a link it did not just make** — a folder already
sharing keeps sharing, whatever else changes.

Sharing is now switchable outright (`sync.enabled`, the config store like
`announcements.enabled`): moving a user's files into a common store is a policy
some people simply do not want, and the honest answer to that is a switch, not a
warning they cannot turn off. Off, no pass runs — and existing links are left
exactly where they are.
