//! `screenshot.*` — every instance's screenshots as one listing.

use proto::screenshot::{
    Screenshot as Shot, ScreenshotDelete, ScreenshotDeleteParams, ScreenshotList,
    ScreenshotListParams,
};

use crate::{IpcError, Session};

pub struct Screenshot<'a> {
    pub(crate) session: &'a Session,
}

impl Screenshot<'_> {
    /// Every instance that shares them; one instance when named.
    pub async fn list(&self, instance: &str) -> Result<Vec<Shot>, IpcError> {
        Ok(self
            .session
            .call::<ScreenshotList>(&ScreenshotListParams {
                instance: instance.to_string(),
            })
            .await?
            .screenshots)
    }

    pub async fn delete(&self, instance: &str, file: &str) -> Result<Vec<Shot>, IpcError> {
        Ok(self
            .session
            .call::<ScreenshotDelete>(&ScreenshotDeleteParams {
                instance: instance.to_string(),
                file: file.to_string(),
            })
            .await?
            .screenshots)
    }
}
