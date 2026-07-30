# Entry-first, with verb-first shortcuts for the hot path

*Applies to: [Front-ends](../architecture/frontends.md)*

The per-entry grammar used to be verb-then-entry, but the entry landed in a
different argument position in every subcommand (`server start smp`, `server
config smp set …`, `server backup create smp`), with no rule for where the name
went — easy to get wrong and hard to remember. Fixing the name to one slot
(`server <name> <action>`) removes that guesswork and lets each per-entry verb
drop its own entry argument. The two exceptions to noun-first are earned, not
sloppy: `play` and the `start`/`stop`/`restart`/`logs`/`rename` shortcuts are
the actions taken often enough that making the user first pick the right noun
(and remember `launch` ≠ `start`) is the friction worth paying a cross-registry
name lookup to avoid. Everything scriptable still has an explicit, unambiguous
noun-first form; the shortcuts are additive sugar over it.
