#!/usr/bin/env bash
# Sourced by dev.sh and run.sh: serve the announcement feed built from news/*.md
# and point this shell's children at it. On by default, off with --no-news.
#
# The *daemon* is what fetches the feed, so the endpoint has to be exported
# before it is spawned — and only a debug build honours the override at all.
# The served feed is unsigned; a debug build on an overridden endpoint waives
# the signature check and never caches what it read.
#
# Sets NEWS_FEED_PID rather than installing its own EXIT trap, because both
# callers already own theirs and the last trap installed would win.

_news_log() {
  printf '%b==>%b %s\n' "${_C:-}" "${_R:-}" "$*" >&2
}

# Being the default means never blocking the dev loop: every failure here drops
# the override and leaves the daemon on the published feed, which is the same
# thing it would have done before any of this existed.
serve_local_feed() {
  local port="${HESTIA_NEWS_PORT:-8787}"
  local url="http://127.0.0.1:$port/announcements.json"

  # A second dev shell finds the first one's server already up. Reuse it rather
  # than failing to bind and exporting an endpoint nothing answers — but rewrite
  # what it serves first, since it compiled news/ when *it* started and the point
  # of this is to see an edit. http.server reads from disk per request, so
  # replacing the file is enough; the other shell still owns the process.
  local feed="target/announce/announcements.json"
  if command -v curl > /dev/null && curl -sf --max-time 1 "$url" > /dev/null 2>&1; then
    if [ -f "$feed" ]; then
      if scripts/announce.sh --envelope > "$feed.new" 2> /dev/null; then
        mv "$feed.new" "$feed"
      else
        rm -f "$feed.new"
        _news_log "news/ does not compile — the feed on $port is left as it was"
      fi
    fi
    export HESTIA_ANNOUNCE_ENDPOINT="$url"
    _news_log "announcement feed already on $port — reusing it"
    return 0
  fi

  # Compile in the foreground: malformed frontmatter is the common failure and
  # its error has to land here, not in a backgrounded job's discarded output.
  local errors
  errors="$(scripts/announce.sh 2>&1 > /dev/null)" || {
    _news_log "news/ does not compile — serving no local feed:"
    printf '%s\n' "$errors" >&2
    return 0
  }

  scripts/announce.sh --serve "$port" > /dev/null 2>&1 &
  NEWS_FEED_PID=$!
  export HESTIA_ANNOUNCE_ENDPOINT="$url"
  _news_log "announcement feed on $url (unsigned — debug builds only)"
}

stop_local_feed() {
  if [ -n "${NEWS_FEED_PID:-}" ]; then
    kill "$NEWS_FEED_PID" 2> /dev/null || true
  fi
}
