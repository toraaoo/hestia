# A warning the user did not cause is a bug in the launcher, not a notice

*Applies to: [The socket boundary](../architecture/wire.md)*

The [structured-warning rule](0029-degraded-outcomes-ride-on-the-result.md)
earns its keep only for outcomes the user can act on. A warning that follows
from hestia's own limitations is noise twice over: it names no action, and the
remediation it points at is work the daemon is declining to do. A NeoForge
create that cannot derive its property schema, or a launch that reports leaving
alone a directory it could perfectly well have moved itself, are both of that
kind.

The answer to one is to **remove the degradation**, not to soften the text. The
schema run resolves its own arguments rather than reporting that it could not.
Sync shares a closed catalogue of files whose format it can merge, so an
ordinary launch has nothing to decline: there is no directory to claim and no
link to refuse, and two instances editing different settings both keep their
change.

What stays a warning is what only the user can settle. Sync reports a unit it
could not read at all, because that instance then runs on its own copy and the
player would otherwise never know. It reports a pre-1.13 instance holding back
its options, because 1.13 renamed every keybind and neither era can read the
other's file in either direction — no launcher can fix that, and the player has
to know which copy they are playing.

Sharing is switchable per unit and per instance for the same reason: keeping a
user's files the same across instances is a policy some people do not want, and
the honest answer to that is a switch, not a warning they cannot turn off. Off,
no pass runs and every instance keeps exactly what it has.

The test for a new warning is therefore to name the action it asks for. When
the answer is "something hestia could do itself", it is a missing feature
wearing a warning's clothes, and the fix belongs in the code it reports on.
