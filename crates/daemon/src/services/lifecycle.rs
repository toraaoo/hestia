//! Liveness, identity, daemon lifecycle, and the event subscription channel.

use proto::app::{AppInfo, AppInfoResult};
use proto::daemon::{DaemonStatus, DaemonStatusResult, DaemonStop, DaemonStopResult};
use proto::events::{EventsSubscribe, EventsSubscribeResult};
use proto::health::{Ping, PingResult};
use proto::Empty;

use crate::runtime::Channels;

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<Ping, _, _>(|_: Empty, _ctx| async move {
        Ok(PingResult {
            status: "alive".into(),
            pid: std::process::id() as i32,
        })
    });

    on.handle::<AppInfo, _, _>(|_: Empty, _ctx| async move {
        Ok(AppInfoResult {
            name: common::app::NAME.into(),
            version: common::app::VERSION_LABEL.into(),
            id: common::app::ID.into(),
            vendor: common::app::VENDOR.into(),
            channel: common::app::CHANNEL.into(),
        })
    });

    on.handle::<DaemonStatus, _, _>(|_: Empty, ctx| async move {
        Ok(DaemonStatusResult {
            pid: std::process::id() as i64,
            version: common::app::VERSION_LABEL.into(),
            uptime_seconds: ctx.runtime.uptime_seconds(),
            home: ctx.runtime.engine().data_home(),
            log: ctx.runtime.log_path().clone(),
        })
    });

    on.handle::<DaemonStop, _, _>(|p, ctx| async move {
        tracing::info!(stop_processes = p.stop_processes, "daemon stop requested");
        // Stop on a short delay so this response reaches the client before the
        // serve loop shuts down.
        let runtime = ctx.runtime.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            runtime.request_stop(p.stop_processes);
        });
        Ok(DaemonStopResult { stopping: true })
    });

    on.handle::<EventsSubscribe, _, _>(|p, ctx| async move {
        let filter = if p.id.is_empty() { None } else { Some(p.id) };
        ctx.runtime
            .hub()
            .subscribe(ctx.conn_id, ctx.out.clone(), filter);
        Ok(EventsSubscribeResult { subscribed: true })
    });
}
