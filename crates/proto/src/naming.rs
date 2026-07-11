//! The naming rules both sides of the socket resolve through, so the CLI and
//! the daemon can never disagree on which entry a bare reference names. Pure
//! functions, no I/O — the same no-drift role `contract` plays for payloads.

/// Reduce a display name to a filesystem-safe slug: lowercase alphanumeric runs
/// joined by single dashes. `None` when the name has no usable characters.
pub fn slugify(name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut gap = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if gap && !slug.is_empty() {
                slug.push('-');
            }
            gap = false;
            slug.push(c.to_ascii_lowercase());
        } else {
            gap = true;
        }
    }
    (!slug.is_empty()).then_some(slug)
}

/// Does `reference` identify the entry with this `id`/`name`? Matches the exact
/// id (`smp-3f9a2c7d`), or any spelling of the display name that slugs the same
/// — so `My Server`, `my-server`, and `MY  SERVER` all resolve to the one
/// server named "My Server". Unambiguous because entry names are slug-unique;
/// an exact id wins over a slugged name.
pub fn reference_matches(reference: &str, id: &str, name: &str) -> bool {
    if id == reference {
        return true;
    }
    matches!(
        (slugify(reference), slugify(name)),
        (Some(a), Some(b)) if a == b
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_normalizes_case_and_punctuation() {
        assert_eq!(slugify("My Server!").as_deref(), Some("my-server"));
        assert_eq!(slugify("  a__b  ").as_deref(), Some("a-b"));
        assert_eq!(slugify("!!!"), None);
    }

    #[test]
    fn reference_matches_id_exact_and_slugged_name() {
        let (id, name) = ("my-server-3f9a2c7d", "My Server");
        assert!(reference_matches(id, id, name), "exact id");
        assert!(reference_matches("My Server", id, name), "exact name");
        assert!(reference_matches("my-server", id, name), "slugged name");
        assert!(reference_matches("MY  SERVER", id, name), "loose spelling");
        assert!(!reference_matches("other", id, name));
    }
}
