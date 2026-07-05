//! Upstream metadata clients shared by the flavor providers: Mojang's
//! piston-meta catalogue and the Fabric Meta API. Fetch + parse only; the
//! providers assemble the results into launch profiles.

pub mod fabric;
pub mod mojang;

use anyhow::{bail, Context, Result};
use serde_json::Value;

async fn fetch_json(url: &str) -> Result<Value> {
    tracing::debug!(url, "minecraft meta GET");
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .with_context(|| format!("request to {url} failed"))?;
    if !response.status().is_success() {
        bail!(
            "request to {url} failed: HTTP {}",
            response.status().as_u16()
        );
    }
    response
        .json()
        .await
        .with_context(|| format!("{url} returned malformed JSON"))
}

/// Host OS in Mojang's rule vocabulary (`os.name`).
fn host_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

/// Evaluate a Mojang `rules` array against the host. No rules ⇒ allowed. Only
/// `os.name` is understood; feature-gated rules (demo mode, custom resolution)
/// are treated as not-applicable and skipped.
fn rules_allow(rules: &Value) -> bool {
    let Some(rules) = rules.as_array() else {
        return true;
    };
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        if rule.get("features").is_some() {
            continue;
        }
        let matches = match rule
            .get("os")
            .and_then(|o| o.get("name"))
            .and_then(Value::as_str)
        {
            Some(name) => name == host_os(),
            None => true,
        };
        if matches {
            allowed = rule
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or("allow")
                == "allow";
        }
    }
    allowed
}

/// Convert a Maven coordinate (`group:artifact:version[:classifier][@ext]`) into
/// its repository-relative path.
fn maven_path(coord: &str) -> Option<String> {
    let (coord, ext) = coord.split_once('@').unwrap_or((coord, "jar"));
    let parts: Vec<&str> = coord.split(':').collect();
    if parts.len() < 3 || parts.iter().take(3).any(|p| p.is_empty()) {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let (artifact, version) = (parts[1], parts[2]);
    let classifier = parts.get(3).map(|c| format!("-{c}")).unwrap_or_default();
    Some(format!(
        "{group}/{artifact}/{version}/{artifact}-{version}{classifier}.{ext}"
    ))
}

fn filename_of(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}
