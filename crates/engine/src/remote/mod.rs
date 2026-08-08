//! The keys that open the node's second door.
//!
//! One document in the data home, owner-only, holding a digest per key and never
//! a key. Minting is deliberately a *local* act — these are reachable over the
//! socket and nowhere else — so a stolen key can never mint its replacement
//! ([0075](../../../docs/decisions/0075-the-remote-surface-is-an-allowlist.md)).

mod key;

use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;
use proto::remote::{RemoteKey, Scope};
use serde::{Deserialize, Serialize};

use crate::registry::{allocate_id, now_unix};
use crate::schema::{self, Document};

/// A key as it is persisted: everything a listing shows, plus the digest that
/// recognises the holder.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
struct StoredKey {
    id: String,
    name: String,
    prefix: String,
    digest: String,
    scopes: Vec<Scope>,
    servers: Vec<String>,
    created_unix: i64,
    last_used_unix: i64,
}

impl StoredKey {
    fn info(&self) -> RemoteKey {
        RemoteKey {
            id: self.id.clone(),
            name: self.name.clone(),
            prefix: self.prefix.clone(),
            scopes: self.scopes.clone(),
            servers: self.servers.clone(),
            created_unix: self.created_unix,
            last_used_unix: self.last_used_unix,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
struct Stored {
    keys: Vec<StoredKey>,
}

impl Document for Stored {
    const NAME: &'static str = "remote-keys.json";
}

/// What a verified key turned out to be allowed to do. Deliberately not the
/// whole record: an authenticated request needs the grant, never the digest.
#[derive(Debug, Clone)]
pub struct Grant {
    pub id: String,
    pub prefix: String,
    pub scopes: Vec<Scope>,
    /// The server ids this key is narrowed to; empty means every server.
    pub servers: Vec<String>,
}

impl Grant {
    pub fn holds(&self, scope: Scope) -> bool {
        self.scopes.contains(&scope)
    }

    /// Whether this key may touch a given server at all. A key narrowed to a set
    /// of ids is invisible to every other server on the node.
    pub fn covers(&self, server_id: &str) -> bool {
        self.servers.is_empty() || self.servers.iter().any(|id| id == server_id)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    #[error("a key needs a name")]
    NameRequired,
    #[error("a key needs at least one scope")]
    ScopesRequired,
    #[error("no remote key matches {0}")]
    NotFound(String),
    #[error(transparent)]
    Save(anyhow::Error),
}

pub struct Remote {
    inner: Mutex<Inner>,
}

struct Inner {
    path: PathBuf,
    stored: Stored,
}

impl Remote {
    pub fn new(path: PathBuf) -> Self {
        Remote {
            inner: Mutex::new(Inner {
                stored: schema::load(&path).unwrap_or_default(),
                path,
            }),
        }
    }

    pub fn reload(&self, path: PathBuf) {
        let mut inner = self.inner.lock().unwrap();
        inner.stored = schema::load(&path).unwrap_or_default();
        tracing::debug!(path = %path.display(), "remote key store reloaded");
        inner.path = path;
    }

    pub fn list(&self) -> Vec<RemoteKey> {
        let inner = self.inner.lock().unwrap();
        inner.stored.keys.iter().map(StoredKey::info).collect()
    }

    pub fn count(&self) -> usize {
        self.inner.lock().unwrap().stored.keys.len()
    }

    /// Mint a key. The plaintext is returned once, here, and is not recoverable
    /// afterwards — only its digest is written.
    pub fn create(
        &self,
        name: &str,
        scopes: Vec<Scope>,
        servers: Vec<String>,
    ) -> Result<(RemoteKey, String), RemoteError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(RemoteError::NameRequired);
        }
        if scopes.is_empty() {
            return Err(RemoteError::ScopesRequired);
        }

        let mut scopes = scopes;
        scopes.sort_unstable();
        scopes.dedup();

        let mut inner = self.inner.lock().unwrap();
        let taken = |id: &str| inner.stored.keys.iter().any(|k| k.id == id);
        let id = allocate_id(taken).map_err(RemoteError::Save)?;
        let issued = key::issue();
        let record = StoredKey {
            id,
            name: name.to_string(),
            prefix: issued.prefix,
            digest: issued.digest,
            scopes,
            servers,
            created_unix: now_unix(),
            last_used_unix: 0,
        };
        let info = record.info();
        inner.stored.keys.push(record);
        save(&mut inner)?;
        tracing::info!(key = %info.prefix, name = %info.name, scopes = info.scopes.len(), "remote key created");
        Ok((info, issued.token))
    }

    /// Revoke by id or by prefix, whichever the caller has in front of it.
    pub fn revoke(&self, reference: &str) -> Result<RemoteKey, RemoteError> {
        let reference = reference.trim();
        let mut inner = self.inner.lock().unwrap();
        let found = inner
            .stored
            .keys
            .iter()
            .position(|k| k.id == reference || k.prefix == reference)
            .ok_or_else(|| RemoteError::NotFound(reference.to_string()))?;
        let removed = inner.stored.keys.remove(found).info();
        save(&mut inner)?;
        tracing::info!(key = %removed.prefix, name = %removed.name, "remote key revoked");
        Ok(removed)
    }

    /// Recognise a presented token, and record that it was used.
    ///
    /// Every stored digest is compared even after one matches: returning as soon
    /// as a key is found would make the answer's timing depend on where in the
    /// store the match sits, which is a slow enumeration of how many keys a node
    /// holds.
    pub fn verify(&self, token: &str) -> Option<Grant> {
        if !key::looks_like_a_key(token) {
            return None;
        }
        let presented = key::digest(token);
        let mut inner = self.inner.lock().unwrap();
        let mut found = None;
        for (index, stored) in inner.stored.keys.iter().enumerate() {
            if key::matches(&stored.digest, &presented) {
                found = Some(index);
            }
        }
        let index = found?;
        inner.stored.keys[index].last_used_unix = now_unix();
        let stored = &inner.stored.keys[index];
        let grant = Grant {
            id: stored.id.clone(),
            prefix: stored.prefix.clone(),
            scopes: stored.scopes.clone(),
            servers: stored.servers.clone(),
        };
        // A last-used stamp is worth losing rather than failing a request that
        // is otherwise perfectly authorized.
        if let Err(e) = save(&mut inner) {
            tracing::warn!(error = %e, "could not record remote key use");
        }
        Some(grant)
    }
}

fn save(inner: &mut Inner) -> Result<(), RemoteError> {
    // Owner-only: the digests are not usable as keys, but the file also names
    // every scope every key holds, which is a map of the door for anyone
    // deciding where to push.
    schema::save_private(&inner.path, &inner.stored).map_err(RemoteError::Save)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, Remote) {
        let dir = tempfile::Builder::new()
            .prefix("hestia-remote-")
            .tempdir()
            .expect("temp dir");
        let remote = Remote::new(dir.path().join("remote-keys.json"));
        (dir, remote)
    }

    #[test]
    fn a_minted_key_verifies_and_carries_its_scopes() {
        let (_dir, remote) = store();
        let (info, token) = remote
            .create(
                "laptop",
                vec![Scope::ServerRead, Scope::ServerControl],
                vec![],
            )
            .expect("create");

        let grant = remote.verify(&token).expect("the minted key verifies");
        assert_eq!(grant.id, info.id);
        assert!(grant.holds(Scope::ServerRead));
        assert!(grant.holds(Scope::ServerControl));
        assert!(!grant.holds(Scope::ServerDelete));
    }

    #[test]
    fn the_token_is_never_persisted() {
        let (dir, remote) = store();
        let (_, token) = remote
            .create("laptop", vec![Scope::ServerRead], vec![])
            .expect("create");

        let on_disk =
            std::fs::read_to_string(dir.path().join("remote-keys.json")).expect("written");
        assert!(
            !on_disk.contains(&token),
            "the plaintext key must not reach the disk"
        );
        assert!(on_disk.contains(&key::digest(&token)));
    }

    #[test]
    fn a_key_survives_a_reload() {
        let (dir, remote) = store();
        let (_, token) = remote
            .create("laptop", vec![Scope::ServerRead], vec![])
            .expect("create");

        let reopened = Remote::new(dir.path().join("remote-keys.json"));
        assert!(reopened.verify(&token).is_some());
    }

    #[test]
    fn a_revoked_key_stops_verifying_immediately() {
        let (_dir, remote) = store();
        let (info, token) = remote
            .create("laptop", vec![Scope::ServerRead], vec![])
            .expect("create");

        remote.revoke(&info.id).expect("revoke");

        assert!(remote.verify(&token).is_none());
        assert!(remote.list().is_empty());
    }

    #[test]
    fn a_key_is_revocable_by_the_prefix_a_listing_shows() {
        let (_dir, remote) = store();
        let (info, token) = remote
            .create("laptop", vec![Scope::ServerRead], vec![])
            .expect("create");

        remote.revoke(&info.prefix).expect("revoke by prefix");

        assert!(remote.verify(&token).is_none());
    }

    #[test]
    fn revoking_something_that_was_never_minted_says_so() {
        let (_dir, remote) = store();
        assert!(matches!(
            remote.revoke("hst_nothing"),
            Err(RemoteError::NotFound(_))
        ));
    }

    #[test]
    fn an_unrelated_token_never_verifies() {
        let (_dir, remote) = store();
        remote
            .create("laptop", vec![Scope::ServerRead], vec![])
            .expect("create");

        for foreign in ["", "hst_", "Bearer something", &key::issue().token] {
            assert!(remote.verify(foreign).is_none(), "{foreign} verified");
        }
    }

    #[test]
    fn a_key_needs_a_name_and_a_scope() {
        let (_dir, remote) = store();
        assert!(matches!(
            remote.create("  ", vec![Scope::ServerRead], vec![]),
            Err(RemoteError::NameRequired)
        ));
        assert!(matches!(
            remote.create("laptop", vec![], vec![]),
            Err(RemoteError::ScopesRequired)
        ));
    }

    #[test]
    fn duplicate_scopes_collapse() {
        let (_dir, remote) = store();
        let (info, _) = remote
            .create(
                "laptop",
                vec![Scope::ServerRead, Scope::ServerRead, Scope::ServerControl],
                vec![],
            )
            .expect("create");
        assert_eq!(info.scopes.len(), 2);
    }

    #[test]
    fn a_narrowed_key_covers_only_the_servers_it_names() {
        let (_dir, remote) = store();
        let (_, token) = remote
            .create("smp only", vec![Scope::ServerRead], vec!["abc".into()])
            .expect("create");

        let grant = remote.verify(&token).expect("verifies");
        assert!(grant.covers("abc"));
        assert!(!grant.covers("def"));
    }

    #[test]
    fn a_node_wide_key_covers_every_server() {
        let (_dir, remote) = store();
        let (_, token) = remote
            .create("everything", vec![Scope::ServerRead], vec![])
            .expect("create");

        let grant = remote.verify(&token).expect("verifies");
        assert!(grant.covers("abc"));
        assert!(grant.covers("def"));
    }

    #[test]
    fn using_a_key_stamps_it() {
        let (_dir, remote) = store();
        let (info, token) = remote
            .create("laptop", vec![Scope::ServerRead], vec![])
            .expect("create");
        assert_eq!(info.last_used_unix, 0);

        remote.verify(&token).expect("verifies");

        let listed = remote.list();
        assert!(listed[0].last_used_unix > 0);
    }

    #[test]
    fn the_store_is_owner_only() {
        let (dir, remote) = store();
        remote
            .create("laptop", vec![Scope::ServerRead], vec![])
            .expect("create");

        let path = dir.path().join("remote-keys.json");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o077, 0, "group and other must have no access");
        }
        assert!(path.exists());
    }
}
