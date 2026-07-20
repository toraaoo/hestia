//! The naming rules both sides of the socket resolve through, so the CLI and
//! the daemon can never disagree on which entry a bare reference names. Pure
//! functions, no I/O — the same no-drift role `contract` plays for payloads.

use serde_json::Value;

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

/// Translate a `config.*` key segment from its kebab-case vocabulary
/// (`jvm-args`) to the camelCase field the settings serialize as (`jvmArgs`).
/// The config keys are a deliberately stable kebab-case CLI vocabulary while
/// every serialized struct — settings included — is camelCase, so the
/// dotted-path get/set navigation translates each segment through here.
/// Single-word segments are unchanged.
pub fn config_key_to_field(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    let mut upper = false;
    for c in segment.chars() {
        if c == '-' || c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Recursively rename object keys from the camelCase serialized form back to the
/// kebab-case `config.*` vocabulary, so `config list` presents the settings tree
/// in the keys a user sets. The per-key inverse of [`config_key_to_field`].
pub fn settings_to_config_keys(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (field_to_config_key(&k), settings_to_config_keys(v)))
                .collect(),
        ),
        Value::Array(items) => {
            Value::Array(items.into_iter().map(settings_to_config_keys).collect())
        }
        other => other,
    }
}

fn field_to_config_key(field: &str) -> String {
    let mut out = String::with_capacity(field.len() + 4);
    for c in field.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
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

    #[test]
    fn config_key_translates_to_camel_field_and_back() {
        assert_eq!(config_key_to_field("jvm-args"), "jvmArgs");
        assert_eq!(config_key_to_field("backup-interval"), "backupInterval");
        assert_eq!(config_key_to_field("memory"), "memory");
        let camel = serde_json::json!({ "defaults": { "jvmArgs": "-Xss1m", "memory": "4G" } });
        assert_eq!(
            settings_to_config_keys(camel),
            serde_json::json!({ "defaults": { "jvm-args": "-Xss1m", "memory": "4G" } })
        );
    }
}
