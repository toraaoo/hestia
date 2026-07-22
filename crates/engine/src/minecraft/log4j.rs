//! A per-session Log4j2 configuration for client launches.
//!
//! An instance can run several sessions at once, all sharing one `data/` game
//! directory — so they would otherwise all write the same `logs/latest.log`,
//! interleaving their output. Pointing each launch at its own generated Log4j2
//! config (`-Dlog4j.configurationFile`) gives every session a private log file
//! that the supervisor can tail independently, and — being a real file the game
//! writes — it survives a daemon restart, unlike a captured stdout pipe.
//!
//! Security: the pattern uses `%m{nolookups}` and the launch also sets
//! `-Dlog4j2.formatMsgNoLookups=true`, so this config never re-opens the Log4Shell
//! message-lookup hole (CVE-2021-44228) on versions whose bundled config Mojang
//! had patched.

use std::path::Path;

/// The JVM system property that points Log4j2 at an external config file.
pub const CONFIG_PROPERTY: &str = "-Dlog4j.configurationFile=";
/// The belt-and-suspenders global kill-switch for message lookups.
pub const NO_LOOKUPS_PROPERTY: &str = "-Dlog4j2.formatMsgNoLookups=true";

/// Render a Log4j2 config that logs to the console and to `log_file`.
pub fn session_config(log_file: &Path) -> String {
    include_str!("../../assets/session_log4j2.xml")
        .replace("@LOG_FILE@", &xml_escape(&log_file.to_string_lossy()))
}

fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_is_log4shell_safe_and_points_at_the_file() {
        let xml = session_config(Path::new("/data/logs/session-3.log"));
        assert!(xml.contains(r#"fileName="/data/logs/session-3.log""#));
        assert!(
            xml.contains("%m{nolookups}"),
            "message lookups must be disabled"
        );
        assert!(!xml.contains("${"), "no interpolation left in the template");
    }

    #[test]
    fn special_characters_in_the_path_are_escaped() {
        let xml = session_config(Path::new("/home/a & b/logs/s.log"));
        assert!(xml.contains("a &amp; b"));
        assert!(!xml.contains("a & b"));
    }
}
