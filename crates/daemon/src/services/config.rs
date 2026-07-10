//! The typed settings store, plus the reserved `home` and `autostart` keys the
//! daemon routes to the path pointer and the login registration.

use engine::ConfigError;
use proto::config::{
    ConfigGet, ConfigGetResult, ConfigList, ConfigListResult, ConfigSet, AUTOSTART_KEY, HOME_KEY,
};
use proto::Empty;
use serde_json::{json, Value};

use crate::autostart;
use crate::runtime::{Channels, ServiceError};

pub(super) fn register(on: &mut Channels<'_>) {
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
            tracing::info!(home = %dir, "data home changed");
            return Ok(Empty {});
        }
        if p.key == AUTOSTART_KEY {
            let Value::Bool(enabled) = p.value else {
                return Err(ServiceError::bad_request("autostart expects a boolean"));
            };
            autostart::set(enabled).map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
            return Ok(Empty {});
        }
        ctx.runtime
            .engine()
            .config()
            .set(&p.key, p.value)
            .map_err(|e| ServiceError::bad_request(e.to_string()))?;
        tracing::info!(key = %p.key, "config updated");
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
}
