//! Every channel the daemon serves, wired onto the router. One `handle::<C>` per
//! channel; handlers reach the daemon's collaborators through `HandlerContext`
//! and return `ServiceError` for a typed failure.

use engine::ConfigError;
use proto::app::{AppInfo, AppInfoResult};
use proto::cache::{
    CacheClear, CacheEntry, CacheInfo, CacheInfoResult, CacheList, CacheListResult, CacheUsage,
};
use proto::config::{
    ConfigGet, ConfigGetResult, ConfigList, ConfigListResult, ConfigSet, AUTOSTART_KEY, HOME_KEY,
};
use proto::daemon::{DaemonStatus, DaemonStatusResult, DaemonStop, DaemonStopResult};
use proto::download::DownloadStart;
use proto::events::{EventsSubscribe, EventsSubscribeResult};
use proto::health::{Ping, PingResult};
use proto::java::{
    JavaInstall, JavaInstallResult, JavaList, JavaListResult, JavaReleases, JavaReleasesResult,
    JavaUninstall,
};
use proto::Empty;
use serde_json::{json, Value};

use crate::autostart;
use crate::runtime::{Channels, Router, ServiceError};

pub fn make_router() -> Router {
    let mut router = Router::default();
    let mut on = Channels::new(&mut router);

    on.handle::<Ping, _, _>(|_: Empty, _ctx| async move {
        Ok(PingResult {
            status: "alive".into(),
            pid: std::process::id() as i32,
        })
    });

    on.handle::<AppInfo, _, _>(|_: Empty, _ctx| async move {
        Ok(AppInfoResult {
            name: common::app::NAME.into(),
            version: common::app::VERSION.into(),
            id: common::app::ID.into(),
            vendor: common::app::VENDOR.into(),
            channel: common::app::CHANNEL.into(),
        })
    });

    on.handle::<DaemonStatus, _, _>(|_: Empty, ctx| async move {
        Ok(DaemonStatusResult {
            pid: std::process::id() as i64,
            version: common::app::VERSION.into(),
            uptime_seconds: ctx.runtime.uptime_seconds(),
            home: ctx.runtime.engine().data_home(),
            log: ctx.runtime.log_path().clone(),
        })
    });

    on.handle::<DaemonStop, _, _>(|_: Empty, ctx| async move {
        // Stop on a short delay so this response reaches the client before the
        // serve loop shuts down.
        let runtime = ctx.runtime.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            runtime.request_stop();
        });
        Ok(DaemonStopResult { stopping: true })
    });

    on.handle::<ConfigGet, _, _>(|p, ctx| async move {
        if p.key == HOME_KEY {
            return Ok(ConfigGetResult {
                value: json!(ctx.runtime.engine().data_home().display().to_string()),
            });
        }
        if p.key == AUTOSTART_KEY {
            return Ok(ConfigGetResult {
                value: json!(autostart::is_enabled()),
            });
        }
        match ctx.runtime.engine().config().get(&p.key) {
            Ok(value) => Ok(ConfigGetResult { value }),
            Err(ConfigError::UnknownKey(m)) => {
                Err(ServiceError::not_found(format!("unknown config key: {m}")))
            }
            Err(e) => Err(ServiceError::handler_error(e.to_string())),
        }
    });

    on.handle::<ConfigSet, _, _>(|p, ctx| async move {
        if p.key == HOME_KEY {
            let Value::String(dir) = p.value else {
                return Err(ServiceError::bad_request("home expects a string"));
            };
            ctx.runtime
                .engine()
                .set_data_home(&dir)
                .map_err(|e| ServiceError::handler_error(e.to_string()))?;
            return Ok(Empty {});
        }
        if p.key == AUTOSTART_KEY {
            let Value::Bool(enabled) = p.value else {
                return Err(ServiceError::bad_request("autostart expects a boolean"));
            };
            autostart::set(enabled).map_err(|e| ServiceError::handler_error(e.to_string()))?;
            return Ok(Empty {});
        }
        ctx.runtime
            .engine()
            .config()
            .set(&p.key, p.value)
            .map_err(|e| ServiceError::bad_request(e.to_string()))?;
        Ok(Empty {})
    });

    on.handle::<ConfigList, _, _>(|_: Empty, ctx| async move {
        let mut entries = ctx.runtime.engine().config().all();
        if let Value::Object(map) = &mut entries {
            map.insert(
                HOME_KEY.into(),
                json!(ctx.runtime.engine().data_home().display().to_string()),
            );
            map.insert(AUTOSTART_KEY.into(), json!(autostart::is_enabled()));
        }
        Ok(ConfigListResult { entries })
    });

    on.handle::<CacheInfo, _, _>(|_: Empty, ctx| async move {
        let cache = ctx.runtime.engine().cache();
        let usage = cache.usage();
        Ok(CacheInfoResult {
            path: cache.dir(),
            usage: CacheUsage {
                entries: usage.entries,
                bytes: usage.bytes,
            },
        })
    });

    on.handle::<CacheList, _, _>(|_: Empty, ctx| async move {
        let entries = ctx
            .runtime
            .engine()
            .cache()
            .entries()
            .into_iter()
            .map(|e| CacheEntry {
                checksum: e.checksum,
                size: e.size,
            })
            .collect();
        Ok(CacheListResult { entries })
    });

    on.handle::<CacheClear, _, _>(|_: Empty, ctx| async move {
        let freed = ctx.runtime.engine().cache().clear();
        Ok(CacheUsage {
            entries: freed.entries,
            bytes: freed.bytes,
        })
    });

    on.handle::<JavaReleases, _, _>(|_: Empty, ctx| async move {
        let releases = ctx
            .runtime
            .engine()
            .java()
            .releases()
            .await
            .map_err(|e| ServiceError::handler_error(e.to_string()))?;
        Ok(JavaReleasesResult { releases })
    });

    on.handle::<JavaList, _, _>(|_: Empty, ctx| async move {
        Ok(JavaListResult {
            runtimes: ctx.runtime.engine().java().installed(),
        })
    });

    on.handle::<JavaInstall, _, _>(|p, ctx| async move {
        if p.major <= 0 {
            return Err(ServiceError::bad_request(
                "major must be a positive integer",
            ));
        }
        match ctx.runtime.java_installs().start(p.major, p.id, p.force) {
            Some(id) => Ok(JavaInstallResult { id }),
            None => Err(ServiceError::bad_request(format!(
                "java {} is already being installed",
                p.major
            ))),
        }
    });

    on.handle::<JavaUninstall, _, _>(|p, ctx| async move {
        if p.major <= 0 {
            return Err(ServiceError::bad_request(
                "major must be a positive integer",
            ));
        }
        if ctx.runtime.engine().java().uninstall(p.major) {
            Ok(Empty {})
        } else {
            Err(ServiceError::not_found(format!(
                "no installed java runtime for major {}",
                p.major
            )))
        }
    });

    on.handle::<DownloadStart, _, _>(|spec, ctx| async move {
        if spec.url.is_empty() {
            return Err(ServiceError::bad_request("download url is empty"));
        }
        let id = ctx.runtime.downloads().start(spec);
        Ok(proto::download::DownloadStartResult { id })
    });

    on.handle::<EventsSubscribe, _, _>(|p, ctx| async move {
        let filter = if p.id.is_empty() { None } else { Some(p.id) };
        ctx.runtime
            .hub()
            .subscribe(ctx.conn_id, ctx.out.clone(), filter);
        Ok(EventsSubscribeResult { subscribed: true })
    });

    router
}
