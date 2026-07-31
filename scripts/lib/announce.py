#!/usr/bin/env python
"""The news/ toolchain: compile the announcement feed, or scaffold an entry.

Driven by scripts/announce.sh, which owns the envelope, the local server and the
signature — this is only the document:

    announce.py compile [news] --base-url URL   # {version, entries} on stdout
    announce.py new "Title" [--severity …]      # write news/<date>-<id>.md

One vocabulary serves both. `Frontmatter` is the `key: value` block and the only
thing that knows its syntax; `Announcement` is one validated entry, whichever
direction it was built from; `Feed` is the directory, and owns what only the set
can know — that an id is unique. `Draft` is the scaffolder, and it finishes by
reading its own output back through `Announcement`, so anything it writes is
something the compiler accepts.

Validation is deliberately strict and loud: an unknown key or a malformed date
fails the build rather than silently publishing an entry that reaches nobody.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

SCHEMA_VERSION = 1

# Every frontmatter key, with the feed field it becomes. Authoring keys are
# kebab-case (the human vocabulary this repo already uses for config keys); the
# document itself is camelCase like every other record here.
FIELDS = {
    "id": "id",
    "severity": "severity",
    "title": "title",
    "link": "link",
    "published": "published",
    "expires": "expires",
    "platforms": "platforms",
    "channels": "channels",
    "min-version": "minVersion",
    "max-version": "maxVersion",
}

REQUIRED = ("id", "title", "published")
SEVERITIES = ("info", "warning", "critical")
PLATFORMS = ("linux", "windows", "macos")

ID_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
IMAGE_RE = re.compile(r"(!\[[^\]]*\]\()(?!https?://|/)([^)]+)(\))")
SLUG_RE = re.compile(r"[^a-z0-9]+")

PLACEHOLDER = "TODO: write the announcement body."


class Error(Exception):
    pass


def now_utc() -> int:
    return int(datetime.now(tz=timezone.utc).timestamp())


class Frontmatter:
    """The `key: value` block at the top of an announcement file.

    The one place that knows the block's syntax, in both directions: `split`
    reads a file into this plus its body, `render` writes one back out.
    """

    def __init__(self, values: dict[str, str], source: str) -> None:
        self.values = values
        self.source = source

    @classmethod
    def split(cls, text: str, source: str) -> tuple[Frontmatter, str]:
        """Everything up to the *first* closing fence, and everything after it.

        Line-wise on purpose: a `---` in the body is a horizontal rule, and
        splitting on the delimiter as a substring would silently drop the rest
        of the announcement at the first one.
        """
        lines = text.splitlines()
        if not lines or lines[0].strip() != "---":
            raise Error(f"{source}: no frontmatter (the file must start with '---')")
        for number, line in enumerate(lines[1:], start=1):
            if line.strip() == "---":
                block = "\n".join(lines[1:number])
                return cls(cls._parse(block, source), source), "\n".join(lines[number + 1 :])
        raise Error(f"{source}: frontmatter is not closed with '---'")

    @staticmethod
    def _parse(block: str, source: str) -> dict[str, str]:
        values: dict[str, str] = {}
        for number, line in enumerate(block.splitlines(), start=2):
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            if ":" not in line:
                raise Error(f"{source}:{number}: not a 'key: value' line: {line!r}")
            key, _, value = line.partition(":")
            key = key.strip()
            if key not in FIELDS:
                known = ", ".join(sorted(FIELDS))
                raise Error(f"{source}:{number}: unknown key {key!r} (known: {known})")
            if key in values:
                raise Error(f"{source}:{number}: {key!r} appears twice")
            values[key] = value.strip()
        return values

    @staticmethod
    def render(values: dict[str, str]) -> str:
        lines = [f"{key}: {value}" for key, value in values.items() if value]
        return "---\n" + "\n".join(lines) + "\n---\n"

    def require(self, *keys: str) -> None:
        for key in keys:
            if not self.values.get(key):
                raise Error(f"{self.source}: {key!r} is required")

    def text(self, key: str, default: str = "") -> str:
        return self.values.get(key, default).strip().strip("'\"")

    def date(self, key: str) -> int:
        value = self.text(key)
        if not value:
            return 0
        if value.isdigit():
            return int(value)
        try:
            parsed = datetime.strptime(value, "%Y-%m-%d")
        except ValueError as exc:
            raise Error(f"{self.source}: {key}: {value!r} is not YYYY-MM-DD or a unix time") from exc
        return int(parsed.replace(tzinfo=timezone.utc).timestamp())

    def list_of(self, key: str, allowed: tuple[str, ...] = ()) -> list[str]:
        value = self.values.get(key, "").strip()
        if not value:
            return []
        if value.startswith("[") and value.endswith("]"):
            value = value[1:-1]
        items = [item.strip().strip("'\"") for item in value.split(",")]
        items = [item for item in items if item]
        for item in items:
            if allowed and item not in allowed:
                raise Error(f"{self.source}: {key}: {item!r} is not one of {', '.join(allowed)}")
        return items


@dataclass(frozen=True)
class Announcement:
    """One validated announcement — everything but its place in the feed."""

    source: str
    id: str
    severity: str
    title: str
    body: str
    published: int
    link: str = ""
    expires: int = 0
    platforms: list[str] = field(default_factory=list)
    channels: list[str] = field(default_factory=list)
    min_version: str = ""
    max_version: str = ""

    @classmethod
    def read(cls, path: Path) -> Announcement:
        matter, body = Frontmatter.split(path.read_text(encoding="utf-8"), path.name)
        matter.require(*REQUIRED)

        entry_id = matter.text("id")
        if not ID_RE.match(entry_id):
            raise Error(
                f"{path.name}: id {entry_id!r} must be lowercase letters, digits and dashes — "
                "it is the permanent dismissal key"
            )

        severity = matter.text("severity") or "info"
        if severity not in SEVERITIES:
            raise Error(
                f"{path.name}: severity {severity!r} is not one of {', '.join(SEVERITIES)}"
            )

        body = body.strip()
        if not body:
            raise Error(f"{path.name}: the body is empty")

        return cls(
            source=path.name,
            id=entry_id,
            severity=severity,
            title=matter.text("title"),
            body=body,
            published=matter.date("published"),
            link=matter.text("link"),
            expires=matter.date("expires"),
            platforms=matter.list_of("platforms", PLATFORMS),
            channels=matter.list_of("channels"),
            min_version=matter.text("min-version"),
            max_version=matter.text("max-version"),
        )

    def expired(self, now: int) -> bool:
        return bool(self.expires) and self.expires <= now

    def entry(self, base_url: str) -> dict:
        return {
            "id": self.id,
            "severity": self.severity,
            "title": self.title,
            "body": self._absolute_images(base_url),
            "link": self.link,
            "published": self.published,
            "expires": self.expires,
            "platforms": self.platforms,
            "channels": self.channels,
            "minVersion": self.min_version,
            "maxVersion": self.max_version,
        }

    def _absolute_images(self, base_url: str) -> str:
        """Point relative image paths at the published asset base.

        Authors write `![shot](images/foo.webp)`; the workflow uploads those
        files beside the feed, so the reference has to become absolute at
        compile time — a launcher has no page-relative context to resolve it
        against.
        """

        def replace(match: re.Match[str]) -> str:
            path = match.group(2).strip()
            if not path.startswith("images/"):
                raise Error(f"{self.source}: image {path!r} must live under news/images/")
            name = path[len("images/") :]
            return f"{match.group(1)}{base_url.rstrip('/')}/{name}{match.group(3)}"

        return IMAGE_RE.sub(replace, self.body)


class Feed:
    """The news/ directory: every announcement in it, and what only the set knows."""

    def __init__(self, directory: Path) -> None:
        if not directory.is_dir():
            raise Error(f"no {directory}/ directory")
        self.directory = directory

    def read_all(self) -> list[Announcement]:
        return [Announcement.read(path) for path in self._files()]

    def owners(self) -> dict[str, str]:
        """Each published id, mapped to the file that owns it.

        Unparseable files are skipped rather than fatal: a draft nobody has
        finished must not stop a new one from being scaffolded beside it.
        """
        owners: dict[str, str] = {}
        for path in self._files():
            try:
                matter, _ = Frontmatter.split(path.read_text(encoding="utf-8"), path.name)
            except Error:
                continue
            if entry_id := matter.text("id"):
                owners[entry_id] = path.name
        return owners

    def compile(self, base_url: str, now: int) -> dict:
        entries = []
        seen: dict[str, str] = {}
        for announcement in self.read_all():
            if announcement.expired(now):
                print(f"  skipping {announcement.source}: expired", file=sys.stderr)
                continue
            # A reused id silently hides a new announcement from everyone who
            # dismissed the old one — the failure direction that matters.
            if announcement.id in seen:
                raise Error(
                    f"{announcement.source}: id {announcement.id!r} "
                    f"already used by {seen[announcement.id]}"
                )
            seen[announcement.id] = announcement.source
            entries.append(announcement.entry(base_url))

        entries.sort(key=lambda e: (-e["published"], e["id"]))
        return {"version": SCHEMA_VERSION, "entries": entries}

    def _files(self) -> list[Path]:
        return [
            path
            for path in sorted(self.directory.glob("*.md"))
            if path.name.upper() != "README.MD"
        ]


class Draft:
    """A new announcement, written into the feed under the naming convention.

    Handwriting the file means re-deriving that convention and the frontmatter
    vocabulary every time, and a slip in either surfaces late: a duplicate id
    only fails at publish, and a *reused* one fails silently for everyone who
    dismissed the first. Both are refused here instead.
    """

    def __init__(self, feed: Feed, options: argparse.Namespace) -> None:
        self.feed = feed
        self.title = options.title or self._ask("title")
        if not self.title:
            raise Error("a title is required")

        self.severity = options.severity or self._ask("severity (info|warning|critical)", "info")
        if self.severity not in SEVERITIES:
            raise Error(f"severity {self.severity!r} is not one of {', '.join(SEVERITIES)}")

        self.published = options.published or datetime.now(tz=timezone.utc).strftime("%Y-%m-%d")
        self.id = options.id or self._slug(self.title)
        if not ID_RE.match(self.id):
            raise Error(
                f"id {self.id!r} must be lowercase letters, digits and dashes — "
                "pass --id when the title does not slugify into one"
            )
        if owner := self.feed.owners().get(self.id):
            raise Error(f"id {self.id!r} is already used by {owner}")

        self.body = sys.stdin.read().strip() if options.body == "-" else (options.body or "").strip()
        self.options = options

    @property
    def path(self) -> Path:
        return self.feed.directory / f"{self.published}-{self.id}.md"

    def write(self) -> Path:
        path = self.path
        if path.exists():
            raise Error(f"{path} already exists")

        matter = Frontmatter.render(
            {
                "id": self.id,
                "severity": self.severity,
                "title": self.title,
                "published": self.published,
                "expires": self.options.expires or "",
                "platforms": self._list("platforms"),
                "channels": self._list("channels"),
                "min-version": self.options.min_version or "",
                "max-version": self.options.max_version or "",
                "link": self.options.link or "",
            }
        )
        path.write_text(f"{matter}\n{self.body or PLACEHOLDER}\n", encoding="utf-8")

        # Read back through the compiler's own parser, so a scaffolded file is
        # never one that fails the build later.
        try:
            if Announcement.read(path).expired(now_utc()):
                raise Error(f"expires {self.options.expires!r} is in the past — nobody would see it")
        except Error:
            path.unlink()
            raise
        return path

    def _list(self, key: str) -> str:
        items = [item.strip() for item in (getattr(self.options, key) or "").split(",")]
        items = [item for item in items if item]
        return f"[{', '.join(items)}]" if items else ""

    @staticmethod
    def _slug(text: str) -> str:
        return SLUG_RE.sub("-", text.lower()).strip("-")

    @staticmethod
    def _ask(label: str, default: str = "") -> str:
        if not sys.stdin.isatty():
            return default
        return input(f"{label} [{default}]: " if default else f"{label}: ").strip() or default


def open_in_editor(path: Path) -> None:
    editor = os.environ.get("VISUAL") or os.environ.get("EDITOR")
    if not editor:
        print("  --edit: neither $VISUAL nor $EDITOR is set", file=sys.stderr)
        return
    subprocess.run([*editor.split(), str(path)], check=False)


def run_compile(options: argparse.Namespace) -> int:
    document = Feed(Path(options.news_dir)).compile(options.base_url, now_utc())
    json.dump(document, sys.stdout, indent=2)
    sys.stdout.write("\n")
    print(f"compiled {len(document['entries'])} announcement(s)", file=sys.stderr)
    return 0


def run_new(options: argparse.Namespace) -> int:
    path = Draft(Feed(Path(options.news_dir)), options).write()
    print(f"wrote {path}", file=sys.stderr)
    if not options.body:
        print("  the body is a placeholder — edit it before committing", file=sys.stderr)
    if options.edit:
        open_in_editor(path)
    return 0


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="announce.py", description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)

    compile_cmd = commands.add_parser("compile", help="compile news/*.md into the feed document")
    compile_cmd.add_argument("news_dir", nargs="?", default="news")
    compile_cmd.add_argument("--base-url", default="", help="where published images are served")
    compile_cmd.set_defaults(run=run_compile)

    new_cmd = commands.add_parser("new", help="scaffold one announcement under news/")
    new_cmd.add_argument("title", nargs="?", help="one line; prompted for when omitted")
    new_cmd.add_argument("--id", help="permanent dismissal key (default: a slug of the title)")
    new_cmd.add_argument("--severity", choices=SEVERITIES, help="default: info")
    new_cmd.add_argument("--published", help="YYYY-MM-DD (default: today, UTC)")
    new_cmd.add_argument("--expires", help="YYYY-MM-DD")
    new_cmd.add_argument("--link", help='a "read more" URL')
    new_cmd.add_argument("--platforms", help=f"comma-separated: {', '.join(PLATFORMS)}")
    new_cmd.add_argument("--channels", help="comma-separated release channels")
    new_cmd.add_argument("--min-version", help="inclusive lower bound on the running build")
    new_cmd.add_argument("--max-version", help="inclusive upper bound")
    new_cmd.add_argument("--body", help="the markdown body, or - to read stdin")
    new_cmd.add_argument("--edit", action="store_true", help="open the file in $EDITOR")
    new_cmd.add_argument("--news-dir", default="news", help="default: news")
    new_cmd.set_defaults(run=run_new)

    return parser.parse_args(argv)


if __name__ == "__main__":
    try:
        options = parse_args(sys.argv[1:])
        sys.exit(options.run(options))
    except Error as exc:
        print(f"error: {exc}", file=sys.stderr)
        sys.exit(1)
