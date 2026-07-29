# A global profile stores project references, never jars

*Applies to: [Content & modpacks](../architecture/content.md)*

A data-home-level profile (`profiles/<name>.json`, a bare array of `{source,
project_id, slug}` — the disk is the registry, the name is the slugged filename)
is a reusable "starter pack" of content: jars are version- and loader-specific,
so each `instance.profile.apply` resolves every reference against the *target*
instance's game version and loader through the ordinary add-content path
(`pick_version`, dependencies included). Applied content becomes an ordinary
pool item with an `origin` tag (`profile:<name>`), so all downstream machinery —
the mirror, backup heal, untracked detection, update — works on it unchanged (an
update preserves the tag; a user re-install clears it, taking ownership). Apply
is **one-shot and additive**: a reference already in the pool is skipped (the
local copy wins), one with no compatible version is a per-item failure the batch
continues past, and de-listed references are never removed (the launch-time
reconcile stays a per-instance-profile concern — `content list` shows the origin
instead). Removing a profile-tagged item locally is refused naming the profile —
it would silently reappear at the next apply; the reference leaves the global
profile instead. The apply runs as a `ContentManager` job under the instance's
in-flight key, publishing the `content.*` topics.
