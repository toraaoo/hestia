# News and notices are one mechanism with a severity dial, not two systems

*Applies to: [The engine](../architecture/engine.md)*

The prior art splits them: MultiMC ships a `notifications.json` (targeted by
platform, release channel and version range, with critical/warning/information
levels) *and* Prism adds an RSS news bar, each with its own fetch, parse and
render path. They were added years apart, and the duplication is the cost.
Hestia carries one `announce` domain: an `Announcement` with a `severity` and a
targeting set, where an untargeted `info` entry *is* news and a version-ranged
`critical` one *is* a notice. Adding a severity is an enum variant; adding a
targeting dimension is a field plus a line in `applies`.

**Targeting stays off the wire.** The engine has already applied it, so
`proto::announce::Announcement` is deliberately narrower than the feed's own
entry — the same rule that put `accepts` on `ServerInfo` rather than letting
each front-end keep a flavor table. A front-end renders what it is given.

**Severity picks the surface, one each**, because intrusiveness should track
urgency (Carbon's notification pattern, and NN/g's heuristic) and because two
surfaces for one announcement means dismissing it twice: `critical` opens a
dialog once per id, `warning` leaves a standing strip until dismissed, `info`
gets the news page and an unread badge. This mirrors what this codebase already
does for daemon warnings — a toast on the operation, a standing `WarningNotice`
on the entry — so the banner is a placement of an existing component, not a new
pattern. Dismissal is daemon-side (`announce/seen.json`), so "once" survives a
restart and the desktop and CLI share one read state; the alternative, desktop
`prefs.json`, would have left the CLI re-nagging about what the UI had already
shown.

**The feed is signed with its own key.** Announcements are display-only text,
which is why no other launcher signs them — but hestia renders remote markdown
with links in the same app that ships an updater, so a hostile endpoint could
phish ("critical: get the hotfix at …"). It reuses the minisign verification the
updater already had (lifted into `engine/signature.rs`), against
`ANNOUNCE_PUBKEY` rather than `UPDATE_PUBKEY`: the announce workflow runs on a
push to the default branch while installers are signed only from a release tag,
so one key would put the installer-signing secret within reach of anything that
can land a commit. A compromised announcement key can say things; it cannot ship
code. **An empty key set fails closed** — a build with no compiled-in key shows
no announcements rather than trusting what it was handed — and the cached
document is re-verified on load, so it is trusted because it verifies *now*, the
same rule the download cache applies when it re-hashes a blob on the way out.

**Publishing is a commit.** `news/*.md` compiles (`scripts/lib/announce.py`) into
one document that CI signs and uploads to a standing `announcements` release tag
— a dedicated tag, because `releases/latest/` would tie news to the release
cadence and 404 on any release that omitted the asset. Validation is strict and
loud: a reused id would silently hide a *new* announcement from everyone who
dismissed the old one, which is the failure direction that matters, so the
compiler refuses duplicates. A malformed version bound likewise reaches nobody
rather than everybody.

Two accepted limits, documented rather than guarded: the signature covers the
feed text, not the images it references by URL (an image can change or 404 after
signing — worst case a wrong picture, never code execution), and
`HESTIA_ANNOUNCE_ENDPOINT` waives the signature so a **debug** build can render
a hand-written feed. That waiver exists only under `cfg(debug_assertions)` (a
release build has no path to it), only for an explicitly overridden endpoint,
logs at WARN, and an unchecked feed is never cached — so it cannot outlive the
process that read it.

The poll is the daemon's one **unprompted** outbound request (the update check
is on demand), which is a real behaviour change for a resident process — so it
is switchable, `announcements.enabled`, and `AnnounceListResult` carries
`enabled` because an empty list means something different when the feed is off
than when nothing is published, and a front-end cannot tell those apart from the
list alone.
