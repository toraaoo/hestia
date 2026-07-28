//! The release notes for *this* build, compiled in from `CHANGELOG.md`.
//!
//! Deliberately bundled rather than fetched. The moment the notes are wanted is
//! the first run after an update, which is exactly when the network may be the
//! thing that just went wrong — and a build's own notes cannot be stale or
//! spoofed if they ship inside it. The announcement feed is the other half:
//! remote, for things learned *after* a release went out.

/// The whole file, embedded at compile time. Small enough that parsing the one
/// section we need at runtime is cheaper than a build script.
const CHANGELOG: &str = include_str!("../../../CHANGELOG.md");

const MARKER: &str = "## ";

/// The notes for `version`, or `None` when the changelog has no section for it.
/// A missing section is not an error: a development build between releases
/// legitimately has none, and the caller shows nothing.
pub fn for_version(version: &str) -> Option<&'static str> {
    let wanted = version.trim().trim_start_matches('v');
    for (index, _) in CHANGELOG.match_indices(MARKER) {
        // Only at the start of a line, so a `### ` sub-heading inside a
        // section's body is body, not the next section.
        if index != 0 && CHANGELOG.as_bytes()[index - 1] != b'\n' {
            continue;
        }
        let after = &CHANGELOG[index + MARKER.len()..];
        let Some((name, body)) = after.split_once('\n') else {
            continue;
        };
        if name.trim().trim_start_matches('v') != wanted {
            continue;
        }
        let notes = body[..next_heading(body)].trim();
        return (!notes.is_empty()).then_some(notes);
    }
    None
}

/// Where the next section starts, or the end of the text.
fn next_heading(text: &str) -> usize {
    text.match_indices("\n## ")
        .next()
        .map_or(text.len(), |(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::for_version;

    #[test]
    fn a_v_prefix_matches_either_way() {
        // Tags carry `v`, `CARGO_PKG_VERSION` does not; neither side should care.
        let bare = for_version(crate::app::VERSION);
        let prefixed = for_version(&format!("v{}", crate::app::VERSION));
        assert!(bare.is_some());
        assert_eq!(bare, prefixed);
    }

    #[test]
    fn a_section_stops_at_the_next_release_not_a_sub_heading() {
        // The real risk in a hand-written changelog: a `### ` inside a section
        // is body, and the section must run to the next `## ` or the end.
        let notes = for_version(crate::app::VERSION).expect("this build has notes");
        assert!(
            !notes.contains("\n## "),
            "a section leaked into the next release: {notes}"
        );
        assert!(
            !notes.starts_with('#'),
            "the heading itself leaked: {notes}"
        );
    }

    #[test]
    fn the_running_version_has_notes() {
        // The changelog must carry a section for whatever this build is, or the
        // what's-new dialog silently shows nothing after an upgrade.
        assert!(
            for_version(crate::app::VERSION).is_some(),
            "CHANGELOG.md has no '## {}' section",
            crate::app::VERSION
        );
    }

    #[test]
    fn an_unreleased_version_has_none() {
        assert!(for_version("99.99.99").is_none());
    }
}
