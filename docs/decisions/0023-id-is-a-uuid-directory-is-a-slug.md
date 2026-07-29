# The id is an opaque uuid; the directory is the slug — decoupled

*Applies to: [Servers & instances](../architecture/entries.md)*

Two facts about an entry pull in opposite directions: its *internal key* must
never change (the supervisor's process key `server-<id>`, the port-claim and
content in-flight key, and how the on-disk `processes/<id>/` records are keyed —
a change orphans a running process and every record pointing at it), while its
*on-disk directory* should read like the entry and track its name. Binding both
to one `<slug>-<suffix>` token forced a choice; splitting them removes it. The
`id` is now a bare UUIDv7 hex string minted once at create — opaque, stable,
never a path component (`registry::allocate_id`). The directory is named
`slugify(name)` (`registry::dir_name`), unique because `name_taken` forbids two
entries slugging alike. So `rename` rewrites the `name` **and moves the
directory** to the new slug, while the id — and everything keyed by it — stays
put; it is guarded stopped-and-not-busy, since no live process may hold the
folder mid-move. A front-end still targets an entry by its **name**, never the
id: a reference resolves by exact id *or* any spelling that slugs to the display
name (`My Server`, `my-server`, `MY  SERVER` all hit the one server named "My
Server"). That rule — `proto::naming::reference_matches` — lives in `proto`, the
same no-drift seam the wire payloads use, so the daemon (`get`) and every
front-end resolve a reference identically; it is unambiguous only because
`name_taken` keeps slugs unique. This is possible cheaply because the id was
never *derived* from the directory name — `registry::scan` deserializes it from
the record JSON — so the folder is just a container the resolvers
(`server_dir`/`data_dir`) name from the record's current name. The rejected
alternative was the old scheme — id *equals* the slug, so it could not move on
rename; the directory then lied about the entry's name forever
(`servers/smp-3f9a2c7d/` lingering after a rename to `cozy`).
