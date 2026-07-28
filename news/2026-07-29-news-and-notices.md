---
id: news-and-notices
severity: info
title: Hestia can now tell you things
published: 2026-07-29
---

Hestia now carries a small news feed. Release notes, breaking changes, and the
occasional notice about a bug worth knowing show up in the launcher instead of
only on a release page you would have to go looking for.

Most of it is quiet — a page and an unread badge. Something that actually costs
you if you miss it, like a data-loss bug in the version you are running, gets a
dialog once and nothing after that.

The feed only ever tells you about the build you are running: an entry can name
a platform and a version range, and you never see the ones that do not apply.
If you would rather it stayed off entirely:

```
hestia config set announcements.enabled false
```

From the terminal, `hestia news` lists what applies to you and
`hestia news refresh` checks immediately rather than waiting for the next poll.
