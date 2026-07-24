//! The typed settings store, plus the reserved `home` and `autostart` keys the
//! daemon routes to the path pointer and the login registration.

use engine::ConfigError;
use proto::config::{
    ConfigGet, ConfigGetResult, ConfigList, ConfigListResult, ConfigSet, AUTOSTART_KEY, HOME_KEY,
};
use proto::error::ErrorInfo;
use proto::Empty;
use serde_json::{json, Value};

use crate::autostart;
use crate::runtime::Channels;

fn config_err(e: ConfigError) -> ErrorInfo {
    match e {
        ConfigError::UnknownKey(key) => ErrorInfo::ConfigKeyUnknown { key },
        ConfigError::TypeMismatch(detail) => ErrorInfo::ConfigTypeMismatch { detail },
        ConfigError::InvalidValue { key, source } => ErrorInfo::ConfigRejected {
            key,
            detail: source.to_string(),
        },
        ConfigError::Rejected { key, message } => ErrorInfo::ConfigRejected {
            key,
            detail: message,
        },
        ConfigError::Io(e) => ErrorInfo::Internal {
            detail: e.to_string(),
        },
    }
}

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
        ctx.runtime
            .engine()
            .config()
            .get(&p.key)
            .map(|value| ConfigGetResult { value })
            .map_err(config_err)
    });

    on.handle::<ConfigSet, _, _>(|p, ctx| async move {
        if p.key == HOME_KEY {
            let Value::String(dir) = p.value else {
                return Err(ErrorInfo::ConfigRejected {
                    key: HOME_KEY.into(),
                    detail: "expects a string".into(),
                });
            };
            ctx.runtime
                .engine()
                .set_data_home(&dir)
                .map_err(|e| ErrorInfo::Internal {
                    detail: e.to_string(),
                })?;
            tracing::info!(home = %dir, "data home changed");
            return Ok(Empty {});
        }
        if p.key == AUTOSTART_KEY {
            let Value::Bool(enabled) = p.value else {
                return Err(ErrorInfo::ConfigRejected {
                    key: AUTOSTART_KEY.into(),
                    detail: "expects a boolean".into(),
                });
            };
            autostart::set(enabled).map_err(crate::runtime::internal)?;
            return Ok(Empty {});
        }
        ctx.runtime
            .engine()
            .config()
            .set(&p.key, p.value)
            .map_err(config_err)?;
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
