use ipc::errors::IpcError;
use proto::profile::{
    GlobalProfile, ProfileCreate, ProfileEdit, ProfileEditParams, ProfileList, ProfileRef,
    ProfileRemove,
};

use crate::session::Session;

/// Global content profiles: data-home-level project reference lists, applied
/// into an instance through `Instance::apply_profile`.
pub struct Profiles<'a> {
    pub(crate) session: &'a Session,
}

impl Profiles<'_> {
    pub async fn list(&self) -> Result<Vec<GlobalProfile>, IpcError> {
        Ok(self
            .session
            .call::<ProfileList>(&proto::Empty {})
            .await?
            .profiles)
    }

    pub async fn create(&self, name: &str) -> Result<GlobalProfile, IpcError> {
        self.session.call::<ProfileCreate>(&profile_ref(name)).await
    }

    pub async fn remove(&self, name: &str) -> Result<(), IpcError> {
        self.session
            .call::<ProfileRemove>(&profile_ref(name))
            .await?;
        Ok(())
    }

    /// Add/remove project references; adds resolve through the content
    /// registry on `source` (empty = the default source), so this can take a
    /// moment per added reference.
    pub async fn edit(
        &self,
        name: &str,
        source: &str,
        add: Vec<String>,
        remove: Vec<String>,
    ) -> Result<GlobalProfile, IpcError> {
        let params = ProfileEditParams {
            name: name.to_string(),
            source: source.to_string(),
            add,
            remove,
        };
        self.session
            .call_with_timeout::<ProfileEdit>(&params, std::time::Duration::from_secs(60))
            .await
    }
}

fn profile_ref(name: &str) -> ProfileRef {
    ProfileRef {
        name: name.to_string(),
    }
}
