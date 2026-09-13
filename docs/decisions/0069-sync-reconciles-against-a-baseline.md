# A synced unit reconciles against a baseline, not a clock

*Applies to: [Servers & instances](../architecture/entries.md)*

Synced files (`options.txt`, `servers.dat`) reconciled **newest-wins**: the
side with the later mtime was taken as the edited one. But an mtime is stamped
by the *copy*, not by the edit behind it — `fs::copy` sets the destination to
`now`, and the `options.txt` merge rewrote both sides on every pass whether or
not anything had changed. So every launch of any instance bumped the store's
stamp, and an instance that had changed nothing outranked one that had. Adding a
server in instance B, launching A, then launching B silently restored B's old
list: B's real edit was older than the stamp A's idle pass had left. The whole
point of sharing is that a change made once is a change everywhere, and this
lost changes instead — quietly, with no failure to notice.

Each store now keeps, per instance, the content that instance and the store last
agreed on (`<store>/.baselines/<instance-id>/<target>`). A pass asks each side
whether it moved since that agreement, which is the question newest-wins was
guessing at: only one side moved, that side wins; neither moved, nothing
happens; both moved, and only then does the clock break the tie — over a copy
that now carries its source's mtime, so it describes the edit rather than the
copy. `options.txt` resolves this **per key**, so two instances changing
different settings both survive, where the whole-file direction used to discard
one. The baseline lives inside the store it describes, so a captured profile
carries its own and `release` takes them with it; `remove_instance` drops what
the shared store held for a deleted instance, the one thing nothing else would
collect.

Two rules fall out and are deliberate. A **missing** side is never an edit: it
is filled from the other, never propagated as a deletion. A key only one side
knows is carried through untouched — instances on different game versions have
different option sets, and treating the absence as a removal would let an old
client strip keys a new one added.

The same baseline is what makes **turning a unit on** expressible. The shared
copy is seeded from one named instance, and every instance's *current* content is
then recorded as its baseline — so every disagreement reads as the shared copy's
change and settles that way, which is what the user asked for by naming the
source. Nothing is deleted to achieve it: an instance that disagrees is simply
brought to the shared copy at its next launch, and a unit it overrides is left
alone entirely.

**Rejected:** hashing content into a sidecar index instead of storing it — the
files are a few KB, and keeping the bytes means the three-way merge has the base
*values* to compare, not just an equal/not-equal answer. **Also rejected:**
propagating deletions, which would make a single instance's absent file delete
everyone's.
