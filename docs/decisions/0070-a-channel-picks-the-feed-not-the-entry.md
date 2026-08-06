# A channel picks the feed, not an entry inside one

*Applies to: [The engine](../architecture/engine.md), [Packaging](../packaging.md)*

Testers need builds before everyone else gets them, and there was one release
manifest — `latest.json`, at
`releases/latest/download/`, holding one version for everybody. A beta had
nowhere to go that did not also hand it to every installed copy.

**The channel is the last path segment of the feed** (`…/updates/stable`,
`…/updates/beta`), and each channel serves the manifest shape that already
existed. `update.channel` in the settings picks which; nothing about the
document changed.

The alternative was one manifest carrying both — `{channels: {stable: …, beta:
…}}`. It reads tidier and costs more: every release of *either* channel has to
republish both entries, so the release workflow either fetches the previous
manifest and merges into it, or a beta publish silently drops the stable entry
and points every stable install at nothing. One document per channel has no such
coupling — a release writes its own feed and cannot corrupt the other.

## Two things had to change underneath it

**Version comparison had to learn prereleases.** `is_newer` compared the numeric
triple with the suffix stripped, so `1.3.0-beta.2` and `1.3.0-beta.1` compared
*equal* — a beta build would never have been offered the next beta, and the
whole channel would have been inert. `version.rs` now implements semver §11
precedence, which also gets the crossing right in both directions: a beta leads
the stable it precedes (`1.3.0-beta.1` > `1.2.5`) and yields to the release it
becomes (`1.3.0` > `1.3.0-beta.9`), so a tester lands on the final build of what
they were testing without doing anything.

**The feed moved off GitHub entirely.** `releases/latest/download/` resolves
only to the newest *non*-prerelease release: it can serve the stable channel and
has no way to name the beta one. Deriving the answer from the releases list
instead — newest non-prerelease for stable, newest of either for beta — would
have worked, and was rejected: it makes the feed a function of GitHub's state,
so the only way to test what a client will be offered is to publish a release.

**CI publishes to the feed; the feed is the record.** The release workflow PUTs
its composed `latest.json` to `/updates/{channel}`, and the API answers exactly
what it was given. Nothing reads tags or releases to decide what is current.
That makes the whole path exercisable without releasing anything — publish a
manifest, poll it back — and it makes a rollback a publish rather than a
retagging exercise. The Release keeps a copy of the manifest as provenance, and
nothing reads it. The contract is written down in
[update-feed.md](../update-feed.md).

The feed is trusted to say *which* version, never to be the source of the bytes:
every artifact is verified against keys compiled into the binary, so a
compromised feed can withhold an update or offer an older signed one, and cannot
cause an unsigned install.

## What follows

- A tag with a semver prerelease suffix (`v1.3.0-beta.1`) is a beta release: the
  workflow publishes it to the beta channel, stamps `HESTIA_CHANNEL=beta` into
  the binaries, and marks the Release prerelease so it never becomes
  `releases/latest` for anyone reading the repository by hand.
- **A build defaults to the channel it shipped on.** A beta build defaulting to
  stable would be stranded — stable does not overtake the prerelease it precedes
  until the *next* version ships — so `common::app::CHANNEL` seeds the setting.
  The user still outranks it; the setting is an ordinary config key.
- Moving from beta back to stable does not downgrade. The rule stays "strictly
  newer, always", so a tester on `1.3.0-beta.2` who switches to stable sits
  there until `1.3.0` ships. Silently reinstalling an older build to honour a
  channel switch is a bigger surprise than waiting.
