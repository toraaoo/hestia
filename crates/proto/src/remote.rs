//! The remote (HTTP) surface as the socket sees it: the scope vocabulary a key
//! carries, and the channels that mint, list and revoke one.
//!
//! These channels are deliberately **not** part of the HTTP allowlist — you
//! cannot mint a key with a key, so provisioning a node is something you do on
//! the node ([0075](../../docs/decisions/0075-the-remote-surface-is-an-allowlist.md)).

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty};

/// What a key is allowed to reach. A route declares the one it costs and the key
/// declares what it holds; the two are compared per route, never inferred from
/// the key being valid.
///
/// The serialized form is the `server:read` vocabulary a user types and a
/// listing prints, so the enum names it explicitly rather than deriving it.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Scope {
    /// Read a server: its record, config, metrics, backups, content and logs.
    #[serde(rename = "server:read")]
    ServerRead,
    /// Start, stop, restart, and send a console command.
    #[serde(rename = "server:control")]
    ServerControl,
    /// Rename it and rewrite its config or content.
    #[serde(rename = "server:write")]
    ServerWrite,
    /// Create, restore and delete its backups.
    #[serde(rename = "server:backup")]
    ServerBackup,
    /// Provision a new server on the node.
    #[serde(rename = "server:create")]
    ServerCreate,
    /// Delete a server and everything under it.
    #[serde(rename = "server:delete")]
    ServerDelete,
}

impl Scope {
    pub const ALL: &'static [Scope] = &[
        Scope::ServerRead,
        Scope::ServerControl,
        Scope::ServerWrite,
        Scope::ServerBackup,
        Scope::ServerCreate,
        Scope::ServerDelete,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Scope::ServerRead => "server:read",
            Scope::ServerControl => "server:control",
            Scope::ServerWrite => "server:write",
            Scope::ServerBackup => "server:backup",
            Scope::ServerCreate => "server:create",
            Scope::ServerDelete => "server:delete",
        }
    }

    pub fn parse(value: &str) -> Option<Scope> {
        Scope::ALL
            .iter()
            .copied()
            .find(|scope| scope.as_str() == value.trim())
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A key as anything but its holder ever sees it: enough to recognise and revoke
/// one, never enough to use it.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default, rename_all = "camelCase")]
pub struct RemoteKey {
    pub id: String,
    pub name: String,
    /// The readable head of the key — `hst_` plus eight characters.
    pub prefix: String,
    pub scopes: Vec<Scope>,
    /// The server ids this key is narrowed to; empty means every server.
    pub servers: Vec<String>,
    pub created_unix: i64,
    /// Unix seconds of the last request this key authenticated, 0 if none has.
    pub last_used_unix: i64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default, rename_all = "camelCase")]
pub struct RemoteKeyCreateParams {
    pub name: String,
    pub scopes: Vec<Scope>,
    /// Narrow the key to these server ids; empty leaves it node-wide.
    pub servers: Vec<String>,
}

/// The one response that carries the secret. It is not stored, so this is the
/// only time it exists anywhere but the holder's hands.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default, rename_all = "camelCase")]
pub struct RemoteKeyCreateResult {
    pub key: RemoteKey,
    /// Shown once and never again — only its digest is kept.
    pub token: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default, rename_all = "camelCase")]
pub struct RemoteKeyListResult {
    pub keys: Vec<RemoteKey>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default, rename_all = "camelCase")]
pub struct RemoteKeyRef {
    /// The key's id, or its prefix — whichever the caller has in front of it.
    pub key: String,
}

/// Whether the door is open, and where. Reported over the socket so an operator
/// can check a node without a key.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default, rename_all = "camelCase")]
pub struct RemoteStatusResult {
    pub enabled: bool,
    /// Where the listener is bound, `host:port`, empty while disabled.
    pub address: String,
    /// Whether the bound address is reachable from off the machine.
    pub exposed: bool,
    /// Why the door did not open, when that is a refusal rather than a setting.
    /// Empty when nothing is wrong.
    pub refusal: String,
    /// The API majors this build serves, newest last.
    pub versions: Vec<String>,
    pub keys: usize,
}

pub struct RemoteStatus;
impl Contract for RemoteStatus {
    const CHANNEL: &'static str = "remote.status";
    type Params = Empty;
    type Result = RemoteStatusResult;
}

pub struct RemoteKeyCreate;
impl Contract for RemoteKeyCreate {
    const CHANNEL: &'static str = "remote.key.create";
    type Params = RemoteKeyCreateParams;
    type Result = RemoteKeyCreateResult;
}

pub struct RemoteKeyList;
impl Contract for RemoteKeyList {
    const CHANNEL: &'static str = "remote.key.list";
    type Params = Empty;
    type Result = RemoteKeyListResult;
}

pub struct RemoteKeyRevoke;
impl Contract for RemoteKeyRevoke {
    const CHANNEL: &'static str = "remote.key.revoke";
    type Params = RemoteKeyRef;
    type Result = Empty;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scope_serializes_as_the_vocabulary_a_user_types() {
        assert_eq!(
            serde_json::to_value(Scope::ServerControl).unwrap(),
            serde_json::json!("server:control")
        );
        let parsed: Scope = serde_json::from_value(serde_json::json!("server:backup")).unwrap();
        assert_eq!(parsed, Scope::ServerBackup);
    }

    #[test]
    fn every_scope_round_trips_through_its_string() {
        for scope in Scope::ALL {
            assert_eq!(Scope::parse(scope.as_str()), Some(*scope));
        }
        assert_eq!(Scope::parse("server:everything"), None);
    }
}
