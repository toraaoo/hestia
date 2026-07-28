//! Comparing the running build against a version string, shared by the
//! self-update check and announcement targeting.
//!
//! Anything unparsable compares as "no answer" (`None`) and every caller treats
//! that as a refusal — a malformed version can neither trigger an update nor
//! decide that an announcement applies.

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

/// Whether `version` falls within an inclusive `[min, max]` range. An empty
/// bound is open; an unparsable one refuses, so a typo in a published range
/// shows the entry to nobody rather than to everybody.
pub fn in_range(version: &str, min: &str, max: &str) -> bool {
    let Some(v) = parse(version) else {
        return false;
    };
    let lower = match min.trim() {
        "" => true,
        bound => parse(bound).is_some_and(|b| v >= b),
    };
    let upper = match max.trim() {
        "" => true,
        bound => parse(bound).is_some_and(|b| v <= b),
    };
    lower && upper
}

#[cfg(test)]
mod tests {
    use super::{in_range, is_newer, parse};

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

    #[test]
    fn an_open_bound_admits_everything() {
        assert!(in_range("0.0.3", "", ""));
        assert!(in_range("9.9.9", "", ""));
    }

    #[test]
    fn range_bounds_are_inclusive() {
        assert!(in_range("0.0.1", "0.0.1", "0.0.3"));
        assert!(in_range("0.0.3", "0.0.1", "0.0.3"));
        assert!(in_range("0.0.2", "0.0.1", "0.0.3"));
        assert!(!in_range("0.0.4", "0.0.1", "0.0.3"));
        assert!(!in_range("0.0.1", "0.0.2", ""));
    }

    #[test]
    fn a_malformed_bound_refuses_rather_than_admits() {
        assert!(!in_range("0.0.2", "oops", ""));
        assert!(!in_range("0.0.2", "", "oops"));
        assert!(!in_range("oops", "", ""));
    }
}
