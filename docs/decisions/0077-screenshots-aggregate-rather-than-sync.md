# Screenshots aggregate rather than sync — nothing is copied

*Applies to: [Instances](../architecture/entries.md#sync--shared-settings-across-instances)*

Screenshots are the one thing in the catalogue with no second copy worth making.
They are written once, never edited, often large, and they belong to the moment
and the instance that took them. Copying them into every instance would multiply
a gigabyte of PNGs across a library for no behaviour the player gains, and
deleting one would then mean deleting it everywhere or nowhere.

So the screenshots unit shares a **view**, not files. Each instance keeps writing
into its own `data/screenshots/`, and the unit decides whether that folder is
read into the one listing the front-ends show — newest first, each shot carrying
the instance it came from. Opening or deleting one acts on the file where it
already is.

This is why the catalogue had to stop meaning "a file we can merge"
([0022](0022-sync-is-a-closed-catalogue.md)): the useful thing here is
aggregation, and a mechanism that only knew how to copy could not express it.

An instance that opts out is simply not read. Nothing moves, so opting in or out
costs nothing and is reversible at any time — unlike every other unit, there is
no baseline, because there is no second copy to disagree with.

**Rejected:** a shared screenshots directory with the instances' folders linked
into it. The game writes into `data/`, which backups and exports already own, and
a link there would make a screenshot appear in an archive of a world it has
nothing to do with. **Also rejected:** copying screenshots into a launcher-owned
pool. It doubles the disk cost of every screenshot to answer a question that
reading the folders answers for free.
