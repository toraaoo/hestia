//! The nodes a control plane knows about. It lives here rather than in either
//! front-end because both are control planes and must read one list
//! ([0074](../../../../docs/decisions/0074-the-node-registry-belongs-to-the-shell.md)).
//! The key is not in `nodes.json`, so a synced copy is worthless on its own.

use std::path::{Path, PathBuf};

use ipc::errors::IpcError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{secrets, Node};

/// A node as everything but the transport sees it — enough to pick one, never
/// enough to open it.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct NodeEntry {
    pub id: String,
    pub label: String,
    /// The node's origin, e.g. `https://prod.example.com`.
    pub url: String,
    /// Unix seconds of the last successful reach, 0 if never.
    pub last_seen_unix: i64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
struct Stored {
    nodes: Vec<NodeEntry>,
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("a node needs a label")]
    LabelRequired,
    #[error("{0} is not an http or https url")]
    BadUrl(String),
    #[error("a node needs a key — mint one on the node with `hestia remote key create`")]
    KeyRequired,
    #[error("no node matches {0}")]
    NotFound(String),
    #[error("a node called {0} is already known")]
    AlreadyKnown(String),
    #[error("cannot write the node list: {0}")]
    Save(String),
}

/// The node list, over one data home.
pub struct Registry {
    home: PathBuf,
}

impl Registry {
    /// Over the resolved data home — the same one the engine and the CLI use, so
    /// `--home` and `$HESTIA_HOME` move the node list with everything else.
    pub fn open(override_home: Option<&Path>) -> Registry {
        Registry {
            home: common::paths::data_home(override_home),
        }
    }

    fn path(&self) -> PathBuf {
        self.home.join("nodes.json")
    }

    pub fn list(&self) -> Vec<NodeEntry> {
        self.load().nodes
    }

    /// Resolve by id or by label, whichever the caller has in front of it.
    pub fn find(&self, reference: &str) -> Option<NodeEntry> {
        let reference = reference.trim();
        self.load()
            .nodes
            .into_iter()
            .find(|node| node.id == reference || node.label == reference)
    }

    pub fn add(&self, label: &str, url: &str, token: &str) -> Result<NodeEntry, RegistryError> {
        let label = label.trim();
        let url = normalize(url)?;
        if label.is_empty() {
            return Err(RegistryError::LabelRequired);
        }
        if token.trim().is_empty() {
            return Err(RegistryError::KeyRequired);
        }
        let mut stored = self.load();
        if stored.nodes.iter().any(|node| node.label == label) {
            return Err(RegistryError::AlreadyKnown(label.to_string()));
        }
        let entry = NodeEntry {
            id: allocate_id(&stored),
            label: label.to_string(),
            url,
            last_seen_unix: 0,
        };
        // The key first: a listed node whose key never landed looks usable.
        secrets::store(&self.home, &entry.id, token.trim()).map_err(RegistryError::Save)?;
        stored.nodes.push(entry.clone());
        self.save(&stored)?;
        tracing::info!(node = %entry.label, "node added");
        Ok(entry)
    }

    pub fn remove(&self, reference: &str) -> Result<NodeEntry, RegistryError> {
        let mut stored = self.load();
        let found = stored
            .nodes
            .iter()
            .position(|node| node.id == reference.trim() || node.label == reference.trim())
            .ok_or_else(|| RegistryError::NotFound(reference.to_string()))?;
        let removed = stored.nodes.remove(found);
        self.save(&stored)?;
        secrets::forget(&self.home, &removed.id);
        tracing::info!(node = %removed.label, "node removed");
        Ok(removed)
    }

    /// Replace a node's key without disturbing the rest of its record — what a
    /// rotation is.
    pub fn rekey(&self, reference: &str, token: &str) -> Result<NodeEntry, RegistryError> {
        if token.trim().is_empty() {
            return Err(RegistryError::KeyRequired);
        }
        let entry = self
            .find(reference)
            .ok_or_else(|| RegistryError::NotFound(reference.to_string()))?;
        secrets::store(&self.home, &entry.id, token.trim()).map_err(RegistryError::Save)?;
        Ok(entry)
    }

    /// Record that a node answered.
    pub fn mark_seen(&self, id: &str) {
        let mut stored = self.load();
        let Some(node) = stored.nodes.iter_mut().find(|node| node.id == id) else {
            return;
        };
        node.last_seen_unix = now_unix();
        let _ = self.save(&stored);
    }

    /// The transport for a node, with its key read at dispatch. The key is never
    /// returned upward: a caller gets something that can *use* it, not the key.
    pub fn connect(&self, reference: &str) -> Result<(NodeEntry, Node), IpcError> {
        let entry = self.find(reference).ok_or_else(|| {
            IpcError::Malformed(RegistryError::NotFound(reference.to_string()).to_string())
        })?;
        let token = secrets::read(&self.home, &entry.id).ok_or_else(|| {
            IpcError::Malformed(format!(
                "no key is stored for '{}' — add it again with its key",
                entry.label
            ))
        })?;
        let node = Node::new(&entry.url, &token);
        Ok((entry, node))
    }

    fn load(&self) -> Stored {
        std::fs::read_to_string(self.path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save(&self, stored: &Stored) -> Result<(), RegistryError> {
        let path = self.path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| RegistryError::Save(e.to_string()))?;
        }
        let text =
            serde_json::to_string_pretty(stored).map_err(|e| RegistryError::Save(e.to_string()))?;
        let part = path.with_extension("json.part");
        std::fs::write(&part, format!("{text}\n"))
            .map_err(|e| RegistryError::Save(e.to_string()))?;
        std::fs::rename(&part, &path).map_err(|e| RegistryError::Save(e.to_string()))
    }
}

/// Only http and https, and only with a host. A `file://` or a bare label would
/// otherwise be stored and fail much later, somewhere less obvious.
fn normalize(url: &str) -> Result<String, RegistryError> {
    let url = url.trim().trim_end_matches('/');
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| RegistryError::BadUrl(url.to_string()))?;
    let host = rest.split('/').next().unwrap_or_default();
    if host.is_empty() || host.starts_with(':') {
        return Err(RegistryError::BadUrl(url.to_string()));
    }
    Ok(url.to_string())
}

/// A time-ordered opaque id, the same shape the engine gives an entry.
fn allocate_id(stored: &Stored) -> String {
    for _ in 0..8 {
        let id = Uuid::now_v7().simple().to_string();
        if !stored.nodes.iter().any(|node| node.id == id) {
            return id;
        }
    }
    Uuid::now_v7().simple().to_string()
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> (tempfile::TempDir, Registry) {
        let dir = tempfile::Builder::new()
            .prefix("hestia-nodes-")
            .tempdir()
            .expect("temp dir");
        let registry = Registry {
            home: dir.path().to_path_buf(),
        };
        (dir, registry)
    }

    #[test]
    fn a_node_is_added_and_found_by_label_or_id() {
        let (_dir, registry) = temp();
        let added = registry
            .add("prod", "https://prod.example.com", "hst_secret")
            .expect("add");

        assert_eq!(registry.find("prod"), Some(added.clone()));
        assert_eq!(registry.find(&added.id), Some(added));
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn the_key_never_reaches_the_node_list() {
        let (dir, registry) = temp();
        registry
            .add("prod", "https://prod.example.com", "hst_secret")
            .expect("add");

        let listed = std::fs::read_to_string(dir.path().join("nodes.json")).expect("written");
        assert!(
            !listed.contains("hst_secret"),
            "a backed-up node list must be worthless on its own"
        );
    }

    #[test]
    fn a_node_survives_a_reopen_and_still_connects() {
        let (dir, registry) = temp();
        registry
            .add("prod", "https://prod.example.com", "hst_secret")
            .expect("add");

        let reopened = Registry {
            home: dir.path().to_path_buf(),
        };
        let (entry, _node) = reopened.connect("prod").expect("connect");
        assert_eq!(entry.url, "https://prod.example.com");
    }

    #[test]
    fn removing_a_node_forgets_its_key_too() {
        let (dir, registry) = temp();
        registry
            .add("prod", "https://prod.example.com", "hst_secret")
            .expect("add");

        registry.remove("prod").expect("remove");

        assert!(registry.list().is_empty());
        assert!(secrets::read(dir.path(), "prod").is_none());
        assert!(registry.connect("prod").is_err());
    }

    #[test]
    fn two_nodes_cannot_share_a_label() {
        let (_dir, registry) = temp();
        registry
            .add("prod", "https://a.example.com", "hst_a")
            .expect("add");
        assert!(matches!(
            registry.add("prod", "https://b.example.com", "hst_b"),
            Err(RegistryError::AlreadyKnown(_))
        ));
    }

    #[test]
    fn a_node_needs_a_label_a_url_and_a_key() {
        let (_dir, registry) = temp();
        assert!(matches!(
            registry.add("  ", "https://a.example.com", "hst_a"),
            Err(RegistryError::LabelRequired)
        ));
        assert!(matches!(
            registry.add("prod", "https://a.example.com", "  "),
            Err(RegistryError::KeyRequired)
        ));
        for bad in [
            "",
            "a.example.com",
            "ftp://a.example.com",
            "https://",
            "file:///etc",
        ] {
            assert!(
                matches!(
                    registry.add("prod", bad, "hst_a"),
                    Err(RegistryError::BadUrl(_))
                ),
                "{bad} was accepted as a url"
            );
        }
    }

    #[test]
    fn a_url_is_stored_without_its_trailing_slash() {
        let (_dir, registry) = temp();
        let added = registry
            .add("prod", "https://prod.example.com/", "hst_a")
            .expect("add");
        assert_eq!(added.url, "https://prod.example.com");
    }

    #[test]
    fn a_rotation_replaces_the_key_and_leaves_the_record() {
        let (dir, registry) = temp();
        let added = registry
            .add("prod", "https://prod.example.com", "hst_old")
            .expect("add");

        registry.rekey("prod", "hst_new").expect("rekey");

        assert_eq!(registry.find("prod"), Some(added.clone()));
        assert_eq!(
            secrets::read(dir.path(), &added.id).as_deref(),
            Some("hst_new")
        );
    }

    #[test]
    fn reaching_a_node_stamps_it() {
        let (_dir, registry) = temp();
        let added = registry
            .add("prod", "https://prod.example.com", "hst_a")
            .expect("add");
        assert_eq!(added.last_seen_unix, 0);

        registry.mark_seen(&added.id);

        assert!(registry.find("prod").unwrap().last_seen_unix > 0);
    }

    #[test]
    fn an_unknown_node_says_so_rather_than_connecting_to_nothing() {
        let (_dir, registry) = temp();
        assert!(registry.connect("nosuch").is_err());
        assert!(matches!(
            registry.remove("nosuch"),
            Err(RegistryError::NotFound(_))
        ));
    }

    #[test]
    fn ids_are_distinct_and_opaque() {
        let (_dir, registry) = temp();
        let a = registry.add("a", "https://a.example.com", "hst_a").unwrap();
        let b = registry.add("b", "https://b.example.com", "hst_b").unwrap();
        assert_ne!(a.id, b.id);
        for id in [&a.id, &b.id] {
            assert_eq!(id.len(), 32);
            assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}
