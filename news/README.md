# Announcements

One markdown file per announcement. Committing to the default branch publishes
it: CI compiles every file here into a single signed document and replaces the
asset on the standing `announcements` release tag, which the launcher polls.

Nothing here is fetched directly — the launcher only ever reads the compiled
`announcements.json`.

## Running it locally

A debug run of either runner serves this directory as the feed and points the
daemon it starts at it — no flag needed:

```bash
scripts/dev.sh                   # subshell with hestia/hestiad on PATH
scripts/run.sh daemon serve
```

Then, from that subshell:

```bash
hestia daemon start
hestia news refresh              # don't wait for the 6-hourly poll
hestia news show <id>
```

`--no-news` skips it, and `HESTIA_NEWS_PORT` moves it off 8787. To drive the
feed by hand instead — or to just check that an entry compiles:

```bash
scripts/announce.sh              # compile + print the payload
scripts/announce.sh --serve      # 127.0.0.1:8787/announcements.json
HESTIA_ANNOUNCE_ENDPOINT=http://127.0.0.1:8787/announcements.json hestiad serve
```

Two things decide whether anything shows up:

- **The endpoint override is read by the daemon, not the CLI** — the daemon is
  what fetches. Export it before `hestia daemon start` so the spawned process
  inherits it. It also only exists in a **debug** build; a release binary has no
  code path to it, which is why a `--release` run never serves one.
- **Announcements are switchable.** If the list is empty, check
  `hestia config get announcements.enabled`.

Serving the feed never fails a run: a `news/` that does not compile, or a port
already taken by something else, is reported and skipped, leaving the daemon on
the published feed.

A served feed is unsigned, and the override waives the signature check
deliberately — requiring one would mean keeping a throwaway keypair just to see
a card render. The waiver is logged at WARN and an unchecked feed is never
cached, so it cannot outlive the daemon that read it.

Publishing is a commit, not a script: CI signs the feed and replaces the release
asset on any push to `main` touching `news/`, and compiles it on a pull request
so a malformed entry fails review instead. `scripts/announce.sh --sign` is what
that workflow runs; it needs the private key.

## Format

```markdown
---
id: rcon-password-log-0-0-3
severity: critical
title: Update to 0.0.4 — rcon passwords could reach the debug log
published: 2026-07-29
expires: 2026-10-01
platforms: [windows]
channels: [dev]
min-version: 0.0.1
max-version: 0.0.3
link: https://github.com/toraaoo/hestia/releases/tag/v0.0.4
---

Builds from 0.0.1 to 0.0.3 wrote a server's rcon password into the firehose
log. Update, then rotate any exposed server's password.
```

| Key | Required | Meaning |
|---|---|---|
| `id` | yes | Lowercase letters, digits, dashes. **Permanent** — see below. |
| `title` | yes | One line, shown in every surface. |
| `published` | yes | `YYYY-MM-DD` (UTC) or a unix time. Sorts the list. |
| `severity` | no | `info` (default), `warning`, `critical`. Picks the surface. |
| `body` | — | Everything after the frontmatter. Markdown. |
| `link` | no | A "read more" URL. |
| `expires` | no | Stops applying after this date, and is dropped at compile. |
| `platforms` | no | `linux`, `windows`, `macos`. Empty = everyone. |
| `channels` | no | Release channels. Empty = every channel. |
| `min-version` | no | Inclusive lower bound on the running build. |
| `max-version` | no | Inclusive upper bound. |

Every targeting key is a filter that an empty value opens, so an announcement
with none of them reaches everyone. A **malformed** version bound reaches
nobody — it fails closed rather than broadcasting a targeted notice to every
build.

## Severity picks the surface

| Severity | Where it appears |
|---|---|
| `critical` | A dialog, once per id, on app open — plus the news page |
| `warning` | A standing banner until dismissed — plus the news page |
| `info` | The news page and the unread badge |

A dialog is worth its interruption only while it stays rare. Reserve
`critical` for something that costs the user if they miss it.

## The id is permanent

It is the dismissal key, so:

- **Renaming** an id resurrects the announcement for everyone who dismissed it.
- **Reusing** an id hides a *new* announcement from everyone who dismissed the
  old one — the dangerous direction, because it fails closed on a real notice.

Date- or version-scoping the id (`rcon-password-log-0-0-3`, not
`security-notice`) makes both mistakes hard to commit by accident. The compiler
refuses two files sharing one id, but it cannot catch a rename.

## Images

Put them in `news/images/` and reference them relatively:

```markdown
![the new news page](images/news-page.webp)
```

The compiler rewrites the path to the published asset URL and CI uploads the
file beside the feed. Note that the signature covers the feed text, not the
image bytes — an image can change or 404 after signing, so do not put anything
load-bearing in one.

## Correcting and retracting

Edit the file and push: same id, so anyone who dismissed it stays dismissed.
Delete the file to retract — clients drop it at their next poll (within six
hours, or immediately on `hestia news refresh`). To force a re-read after a
substantive correction, publish under a new id.
