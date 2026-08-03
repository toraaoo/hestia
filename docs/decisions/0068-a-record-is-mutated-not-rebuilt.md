# A record of the user's is mutated, never rebuilt — and where it must be built, it is built from owned parts

*Applies to: [Content & modpacks](../architecture/content.md), [Servers & instances](../architecture/entries.md)*

Updating an installed mod blanked its icon. The cause was not the icon: the
update flow did not *move* the item to a new version, it built a whole new
`InstalledContent` and put it in the old one's place. To do that it needed a
`ContentProject`, and an update has none — it resolves versions, not projects —
so it fabricated a stub from the index entry and left `icon_url` at its default.

Everything else the record carried went the same way, and the icon was the
visible half. The rebuilt record came back `enabled: true` with an empty
`origin` and no `disabled_worlds`, and the record is what `install::apply_files`
reads to decide where the file is mirrored. So updating a *disabled* mod put it
back in `mods/`. The flow patched `origin` and `enabled` back onto the index
afterwards, which is why the index looked right while the disk did not, and why
the bug read as cosmetic for as long as it did. `merge_pack_items` had the same
shape: a re-applied modpack re-enabled an item the user had turned off.

The patch-up was the tell. Two fields were restored because two fields had been
noticed; the other two had not, and a third added tomorrow would not be either.
A rebuild resets by omission, and omission does not announce itself.

## The rule

**A persisted record of the user's is updated by mutating the loaded document.**
Load it, assign what changed, write it back. This is what the entry stores
already did — `Servers::update` and `Instances::update` assign `profile` and
`phase` onto a record read from disk, and a server keeps its memory, its backup
schedule and the port players connect to across a version change for no reason
other than that nothing else was assigned.

**Where a flow genuinely must build one, it builds it from parts that name their
owner.** Content is that case: an install has a project and a version fetched
upstream and no prior record, an update has a record and a version but no
project, a modpack re-supply has all three plus an item the entry is already
holding a particular way. There is no single "build a record" that serves all
three, so `content::record` splits the fields three ways —

| Group | Owns | Written by |
|---|---|---|
| `Project` | what the item *is* — id, slug, title, icon | an install; carried by everything else |
| `Release` | what it is *at* — source, version, filename, sha1, url | an install, an update, a re-supply |
| `Holding` | how the *entry* holds it — origin, enabled, per-world disables, targeting | the entry, and nothing upstream |

— and `assemble(Project, Release, Holding)` is the only place a record's fields
are written. An update is `repin(item, release)`: project and holding are read
back off the record, so there is no argument through which they could be lost.
A re-supply is `rehold(item, holding)`, with the pack given a say in ownership
alone.

## Why the split is compiler-enforced

`assemble` is exhaustive — a struct literal with no `..Default::default()`. A
field added to `InstalledContent` fails to compile until it has been put in one
of the three groups, which is the whole point: the original bug was a field
nobody classified, defaulting quietly. A test cannot catch the field that does
not exist yet; the build can.

That is also why the rule is not "add part-structs everywhere". The guarantee is
worth its ceremony where a flow rebuilds a record that mixes owners. It buys
nothing where a record has one owner or is never rebuilt, and the audit that
followed this bug found the rest of the engine in that position: a
`JavaRuntime` is discovered by scanning `java/` and is derived end to end; the
download cache is content-addressed; `InstalledModpack` is rebuilt on every
apply and correctly so, being pack-derived throughout; the entry stores mutate.
Wrapping those in an assembler would cost readability to guard a path that does
not exist.

## What was rejected

**Restoring the fields after the rebuild.** What the code already did. It works
until the next field, and gives no signal when it stops working.

**Refetching the project during an update, so a stub is never needed.** It would
have fixed the icon, and it is a network call per item on a path
(`check_entry_updates`) that already walks every installed item and is called by
the desktop's content page. A 40-mod instance would double its request count on
a routine screen to re-learn facts it already had on disk.

**Splitting the entry record into separate documents on disk**, so a
re-provision physically cannot open the user's settings. The strongest version
of the rule, and disproportionate here: the record travels in the transfer
manifest, hestia archives, `.mrpack` and Prism import, so the change reaches all
of them and the schema chain, to guard a rebuild that no flow performs. If a
"repair entry" flow ever re-resolves a profile from scratch, this is the first
thing to revisit.
