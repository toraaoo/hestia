//! Which HTTP route serves which channel.
//!
//! The daemon mounts the routes; this says how to reach them by channel name, so
//! a front-end drives a remote node through the same contracts it uses over the
//! socket. `crates/daemon/tests/remote.rs` walks this table against a running
//! node, so an entry that stops matching a mount fails the build's tests rather
//! than a user's request.

use proto::backup::{
    ServerBackupCreate, ServerBackupList, ServerBackupRemove, ServerBackupRestore,
};
use proto::content::ServerContentList;
use proto::health::Ping;
use proto::server::{
    ServerConfigList, ServerConfigSet, ServerDetail, ServerList, ServerLogs, ServerPing,
    ServerRename, ServerRestart, ServerStart, ServerStatus, ServerStop,
};
use proto::Contract;

pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    /// Whether the leftover payload fields ride in a body or in the query.
    pub fn takes_a_body(&self) -> bool {
        !matches!(self, Method::Get | Method::Delete)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
        }
    }
}

pub struct Route {
    pub channel: &'static str,
    pub method: Method,
    /// The path under `/api/<version>`, with `{placeholder}` segments.
    pub path: &'static str,
    /// Which payload field fills each placeholder, in the order they appear.
    /// A field named here is spent on the path and never repeated in the body.
    pub params: &'static [&'static str],
}

/// Every channel the HTTP surface serves. Servers only — an unlisted channel is
/// not reachable remotely, which is the same rule the daemon's mount enforces.
pub const ROUTES: &[Route] = &[
    Route {
        channel: Ping::CHANNEL,
        method: Method::Get,
        path: "/health",
        params: &[],
    },
    Route {
        channel: ServerList::CHANNEL,
        method: Method::Get,
        path: "/servers",
        params: &[],
    },
    Route {
        channel: ServerStatus::CHANNEL,
        method: Method::Get,
        path: "/servers/{server}",
        params: &["server"],
    },
    Route {
        channel: ServerDetail::CHANNEL,
        method: Method::Get,
        path: "/servers/{server}/detail",
        params: &["server"],
    },
    Route {
        channel: ServerPing::CHANNEL,
        method: Method::Get,
        path: "/servers/{server}/ping",
        params: &["server"],
    },
    Route {
        channel: ServerLogs::CHANNEL,
        method: Method::Get,
        path: "/servers/{server}/logs",
        params: &["server"],
    },
    Route {
        channel: ServerConfigList::CHANNEL,
        method: Method::Get,
        path: "/servers/{server}/config",
        params: &["server"],
    },
    Route {
        channel: ServerContentList::CHANNEL,
        method: Method::Get,
        path: "/servers/{server}/content",
        params: &["server"],
    },
    Route {
        channel: ServerBackupList::CHANNEL,
        method: Method::Get,
        path: "/servers/{server}/backups",
        params: &["server"],
    },
    Route {
        channel: ServerStart::CHANNEL,
        method: Method::Post,
        path: "/servers/{server}/start",
        params: &["server"],
    },
    Route {
        channel: ServerStop::CHANNEL,
        method: Method::Post,
        path: "/servers/{server}/stop",
        params: &["server"],
    },
    Route {
        channel: ServerRestart::CHANNEL,
        method: Method::Post,
        path: "/servers/{server}/restart",
        params: &["server"],
    },
    Route {
        channel: proto::server::ServerCommand::CHANNEL,
        method: Method::Post,
        path: "/servers/{server}/console",
        params: &["server"],
    },
    Route {
        channel: ServerRename::CHANNEL,
        method: Method::Patch,
        path: "/servers/{server}",
        params: &["server"],
    },
    Route {
        channel: ServerConfigSet::CHANNEL,
        method: Method::Put,
        path: "/servers/{server}/config/{key}",
        params: &["server", "key"],
    },
    Route {
        channel: ServerBackupCreate::CHANNEL,
        method: Method::Post,
        path: "/servers/{server}/backups",
        params: &["server"],
    },
    Route {
        channel: ServerBackupRestore::CHANNEL,
        method: Method::Post,
        path: "/servers/{server}/backups/{backup}/restore",
        params: &["server", "backup"],
    },
    Route {
        channel: ServerBackupRemove::CHANNEL,
        method: Method::Delete,
        path: "/servers/{server}/backups/{backup}",
        params: &["server", "backup"],
    },
];

pub fn of(channel: &str) -> Option<&'static Route> {
    ROUTES.iter().find(|route| route.channel == channel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_route_names_a_distinct_channel() {
        let mut seen: Vec<&str> = ROUTES.iter().map(|r| r.channel).collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(before, seen.len(), "a channel is routed twice");
    }

    #[test]
    fn every_placeholder_is_filled_by_a_named_param() {
        for route in ROUTES {
            let holes: Vec<&str> = route
                .path
                .split('{')
                .skip(1)
                .filter_map(|rest| rest.split('}').next())
                .collect();
            assert_eq!(
                holes, route.params,
                "{} declares params that do not match its path",
                route.channel
            );
        }
    }

    /// Nothing outside the server surface is reachable, which is the rule the
    /// daemon's mount enforces from the other side.
    #[test]
    fn nothing_off_the_server_surface_is_routed() {
        for route in ROUTES {
            let channel = route.channel;
            assert!(
                !channel.starts_with("instance.")
                    && !channel.starts_with("account.")
                    && !channel.starts_with("skin.")
                    && !channel.starts_with("sync.")
                    && !channel.starts_with("update.")
                    && !channel.starts_with("remote.")
                    && !channel.starts_with("config.")
                    && !channel.starts_with("daemon."),
                "{channel} must not be reachable over http"
            );
        }
    }
}
