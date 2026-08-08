//! The keys that open the HTTP door, and the state of the door itself.
//!
//! Every channel here is reachable over the socket and over nothing else: they
//! are absent from the HTTP allowlist by construction, so a key can never mint,
//! list or revoke a key
//! ([0075](../../../../docs/decisions/0075-the-remote-surface-is-an-allowlist.md)).

use engine::RemoteError;
use proto::error::{ErrorInfo, Field};
use proto::remote::{
    RemoteKeyCreate, RemoteKeyCreateResult, RemoteKeyList, RemoteKeyListResult, RemoteKeyRevoke,
    RemoteStatus, RemoteStatusResult,
};
use proto::Empty;

use crate::http;
use crate::runtime::Channels;

fn remote_err(e: RemoteError) -> ErrorInfo {
    match e {
        RemoteError::NameRequired => ErrorInfo::FieldRequired { field: Field::Name },
        RemoteError::ScopesRequired => ErrorInfo::FieldRequired {
            field: Field::Scope,
        },
        RemoteError::NotFound(reference) => ErrorInfo::RemoteKeyNotFound { reference },
        RemoteError::Save(e) => ErrorInfo::Internal {
            detail: format!("{e:#}"),
        },
    }
}

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<RemoteStatus, _, _>(|_: Empty, ctx| async move {
        let engine = ctx.runtime.engine();
        let remote = engine.config().settings().remote;
        // Where the listener actually opened, not where the settings asked it
        // to: a refused bind and a port the OS picked both differ, and an
        // operator checking a node needs the truth rather than the intent.
        let door = ctx.runtime.remote_door();
        Ok(RemoteStatusResult {
            enabled: remote.enabled,
            address: door.address,
            exposed: remote.exposed(),
            refusal: door.refusal,
            versions: http::VERSIONS.iter().map(|v| v.to_string()).collect(),
            keys: engine.remote().count(),
        })
    });

    on.handle::<RemoteKeyCreate, _, _>(|p, ctx| async move {
        let (key, token) = ctx
            .runtime
            .engine()
            .remote()
            .create(&p.name, p.scopes, p.servers)
            .map_err(remote_err)?;
        Ok(RemoteKeyCreateResult { key, token })
    });

    on.handle::<RemoteKeyList, _, _>(|_: Empty, ctx| async move {
        Ok(RemoteKeyListResult {
            keys: ctx.runtime.engine().remote().list(),
        })
    });

    on.handle::<RemoteKeyRevoke, _, _>(|p, ctx| async move {
        let revoked = ctx
            .runtime
            .engine()
            .remote()
            .revoke(&p.key)
            .map_err(remote_err)?;
        // Revocation is immediate: a stream opened with this key is dropped
        // rather than left running until its client happens to disconnect.
        ctx.runtime.hub().drop_key(&revoked.id);
        Ok(Empty {})
    });
}
