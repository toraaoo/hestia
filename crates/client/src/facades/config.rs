use ipc::errors::IpcError;
use serde_json::Value;

use crate::session::Session;

pub struct Config<'a> {
    pub(crate) session: &'a Session,
}

impl Config<'_> {
    /// Returns `None` when the key is unknown (a `not_found` from the daemon).
    pub async fn get(&self, key: &str) -> Result<Option<Value>, IpcError> {
        let params = proto::config::ConfigGetParams {
            key: key.to_string(),
        };
        Ok(self
            .session
            .try_call::<proto::config::ConfigGet>(&params)
            .await?
            .map(|r| r.value))
    }

    pub async fn set(&self, key: &str, value: Value) -> Result<(), IpcError> {
        let params = proto::config::ConfigSetParams {
            key: key.to_string(),
            value,
        };
        self.session
            .call::<proto::config::ConfigSet>(&params)
            .await?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Value, IpcError> {
        Ok(self
            .session
            .call::<proto::config::ConfigList>(&proto::Empty {})
            .await?
            .entries)
    }
}
