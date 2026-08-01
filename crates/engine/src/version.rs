//! Comparing the running build against a version string, for the self-update
//! check.
//!
//! Anything unparsable compares as "no answer" (`None`) and every caller treats
//! that as a refusal — a malformed version cannot trigger an update.

/// The numeric `x.y.z` triple, ignoring a `v` prefix and any
/// prerelease/build suffix. `1.2` fills the missing patch with 0.
pub fn parse(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.trim().trim_start_matches('v');
    let v = v.split(['-', '+']).next()?;
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

/// Strictly newer on the numeric triple; anything unparsable is never newer,
/// so a malformed manifest cannot trigger an update.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse(candidate), parse(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_newer, parse};

    #[test]
    fn versions_parse_with_prefixes_and_prereleases() {
        assert_eq!(parse("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse("v0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse("1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(parse("1.2"), Some((1, 2, 0)));
        assert_eq!(parse("not-a-version"), None);
    }

    #[test]
    fn newer_is_strict_and_rejects_garbage() {
        assert!(is_newer("0.0.2", "0.0.1"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.0.1", "0.0.1"));
        assert!(!is_newer("0.0.1", "0.0.2"));
        assert!(!is_newer("garbage", "0.0.1"));
        assert!(!is_newer("0.0.2", "garbage"));
    }
}
