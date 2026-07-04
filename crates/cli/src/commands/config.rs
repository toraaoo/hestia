//! `hestia config …` — typed settings plus the reserved home/autostart keys.

use anyhow::{bail, Result};
use clap::Subcommand;
use serde_json::Value;

use crate::output::print_table;

#[derive(Subcommand)]
pub enum ConfigCmd {
    /// Print the value of a config key
    Get { key: String },
    /// Set the value of a config key
    Set { key: String, value: String },
    /// List all config entries
    List,
}

pub async fn run(cmd: ConfigCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        ConfigCmd::Get { key } => match client.config().get(&key).await? {
            Some(Value::String(s)) => println!("{s}"),
            Some(value) => println!("{}", serde_json::to_string_pretty(&value)?),
            None => bail!("unknown config key: {key}"),
        },
        ConfigCmd::Set { key, value } => {
            // Parse as JSON, falling back to a bare string.
            let parsed = serde_json::from_str::<Value>(&value).unwrap_or(Value::String(value));
            client.config().set(&key, parsed).await?;
        }
        ConfigCmd::List => {
            let entries = client.config().list().await?;
            let mut rows = Vec::new();
            flatten(&entries, "", &mut rows);
            print_table(&["KEY", "VALUE"], &rows);
        }
    }
    Ok(())
}

/// Flatten a settings tree into KEY/VALUE rows, sub-objects as dotted paths.
fn flatten(node: &Value, prefix: &str, rows: &mut Vec<Vec<String>>) {
    if let Value::Object(map) = node {
        for (key, value) in map {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            flatten(value, &path, rows);
        }
        return;
    }
    let rendered = match node {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    rows.push(vec![prefix.to_string(), rendered]);
}
