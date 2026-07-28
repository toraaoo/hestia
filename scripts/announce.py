#!/usr/bin/env python3
"""Compile news/*.md into the announcement feed document.

Each markdown file is one announcement: YAML-ish frontmatter becomes the typed
fields the engine filters on, and the body becomes the markdown a front-end
renders. The result is `{version, entries}` — the payload that gets signed and
wrapped in an envelope by scripts/announce.sh.

Validation is deliberately strict and loud: an unknown key or a malformed date
fails the build rather than silently publishing an entry that reaches nobody.
"""

from __future__ import annotations

import json
import re
import sys
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


class Error(Exception):
    pass


def split_frontmatter(text: str, source: str) -> tuple[dict[str, str], str]:
    if not text.startswith("---"):
        raise Error(f"{source}: no frontmatter (the file must start with '---')")
    parts = text.split("\n---", 2)
    if len(parts) < 2:
        raise Error(f"{source}: frontmatter is not closed with '---'")
    return parse_frontmatter(parts[0][3:], source), parts[1].lstrip("\n")


def parse_frontmatter(block: str, source: str) -> dict[str, str]:
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


def parse_list(value: str, source: str, key: str, allowed: tuple[str, ...]) -> list[str]:
    value = value.strip()
    if not value:
        return []
    if value.startswith("[") and value.endswith("]"):
        value = value[1:-1]
    items = [item.strip().strip("'\"") for item in value.split(",")]
    items = [item for item in items if item]
    for item in items:
        if allowed and item not in allowed:
            raise Error(f"{source}: {key}: {item!r} is not one of {', '.join(allowed)}")
    return items


def parse_date(value: str, source: str, key: str) -> int:
    value = value.strip().strip("'\"")
    if not value:
        return 0
    if value.isdigit():
        return int(value)
    try:
        parsed = datetime.strptime(value, "%Y-%m-%d")
    except ValueError as exc:
        raise Error(f"{source}: {key}: {value!r} is not YYYY-MM-DD or a unix time") from exc
    return int(parsed.replace(tzinfo=timezone.utc).timestamp())


def rewrite_images(body: str, base_url: str, source: str) -> str:
    """Point relative image paths at the published asset base.

    Authors write `![shot](images/foo.webp)`; the workflow uploads those files
    beside the feed, so the reference has to become absolute at compile time —
    a launcher has no page-relative context to resolve it against.
    """

    def replace(match: re.Match[str]) -> str:
        path = match.group(2).strip()
        if not path.startswith("images/"):
            raise Error(f"{source}: image {path!r} must live under news/images/")
        name = path[len("images/") :]
        return f"{match.group(1)}{base_url.rstrip('/')}/{name}{match.group(3)}"

    return IMAGE_RE.sub(replace, body)


def compile_entry(path: Path, base_url: str, now: int) -> dict | None:
    source = path.name
    values, body = split_frontmatter(path.read_text(encoding="utf-8"), source)

    for key in REQUIRED:
        if not values.get(key):
            raise Error(f"{source}: {key!r} is required")

    entry_id = values["id"].strip().strip("'\"")
    if not ID_RE.match(entry_id):
        raise Error(
            f"{source}: id {entry_id!r} must be lowercase letters, digits and dashes — "
            "it is the permanent dismissal key"
        )

    severity = values.get("severity", "info").strip() or "info"
    if severity not in SEVERITIES:
        raise Error(f"{source}: severity {severity!r} is not one of {', '.join(SEVERITIES)}")

    expires = parse_date(values.get("expires", ""), source, "expires")
    if expires and expires <= now:
        print(f"  skipping {source}: expired", file=sys.stderr)
        return None

    body = rewrite_images(body.strip(), base_url, source)
    if not body:
        raise Error(f"{source}: the body is empty")

    return {
        "id": entry_id,
        "severity": severity,
        "title": values["title"].strip().strip("'\""),
        "body": body,
        "link": values.get("link", "").strip().strip("'\""),
        "published": parse_date(values["published"], source, "published"),
        "expires": expires,
        "platforms": parse_list(values.get("platforms", ""), source, "platforms", PLATFORMS),
        "channels": parse_list(values.get("channels", ""), source, "channels", ()),
        "minVersion": values.get("min-version", "").strip().strip("'\""),
        "maxVersion": values.get("max-version", "").strip().strip("'\""),
    }


def main() -> int:
    news_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("news")
    base_url = sys.argv[2] if len(sys.argv) > 2 else ""
    if not news_dir.is_dir():
        print(f"no {news_dir}/ directory", file=sys.stderr)
        return 1

    now = int(datetime.now(tz=timezone.utc).timestamp())
    entries = []
    seen: dict[str, str] = {}
    for path in sorted(news_dir.glob("*.md")):
        if path.name.upper() == "README.MD":
            continue
        entry = compile_entry(path, base_url, now)
        if entry is None:
            continue
        # A reused id silently hides a new announcement from everyone who
        # dismissed the old one — the failure direction that matters.
        if entry["id"] in seen:
            raise Error(f"{path.name}: id {entry['id']!r} already used by {seen[entry['id']]}")
        seen[entry["id"]] = path.name
        entries.append(entry)

    entries.sort(key=lambda e: (-e["published"], e["id"]))
    json.dump({"version": SCHEMA_VERSION, "entries": entries}, sys.stdout, indent=2)
    sys.stdout.write("\n")
    print(f"compiled {len(entries)} announcement(s)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Error as exc:
        print(f"error: {exc}", file=sys.stderr)
        sys.exit(1)
