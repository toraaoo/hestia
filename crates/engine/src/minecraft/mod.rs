//! Minecraft provider aggregate: the server and instance (client) flavor
//! registries and the flavors/versions/resolve entry points over them. Stateless
//! (every result is fetched from upstream), so it needs no data directory.

mod fabric;
pub(crate) mod launch;
pub(crate) mod materialize;
mod meta;
mod provider;
mod vanilla;

use anyhow::{Context, Result};
use proto::minecraft::{Flavor, GameVersion, InstanceProfile, ServerProfile};

use provider::{InstanceProvider, ResolveRequest, ServerProvider};

pub struct Minecraft {
    servers: Vec<Box<dyn ServerProvider>>,
    instances: Vec<Box<dyn InstanceProvider>>,
}

impl Default for Minecraft {
    fn default() -> Self {
        Minecraft {
            servers: vec![
                Box::new(vanilla::VanillaServer),
                Box::new(fabric::FabricServer),
            ],
            instances: vec![
                Box::new(vanilla::VanillaInstance),
                Box::new(fabric::FabricInstance),
            ],
        }
    }
}

impl Minecraft {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn server_flavors(&self) -> Vec<Flavor> {
        self.servers
            .iter()
            .map(|p| flavor(p.id(), p.name()))
            .collect()
    }

    pub fn instance_flavors(&self) -> Vec<Flavor> {
        self.instances
            .iter()
            .map(|p| flavor(p.id(), p.name()))
            .collect()
    }

    pub async fn server_versions(&self, flavor: &str) -> Result<Vec<GameVersion>> {
        self.server(flavor)?.versions().await
    }

    pub async fn resolve_server(
        &self,
        flavor: &str,
        version: &str,
        loader_version: Option<String>,
    ) -> Result<ServerProfile> {
        tracing::info!(flavor, version, ?loader_version, "resolving server profile");
        self.server(flavor)?
            .resolve(&ResolveRequest {
                version: version.to_string(),
                loader_version,
            })
            .await
    }

    pub async fn instance_versions(&self, flavor: &str) -> Result<Vec<GameVersion>> {
        self.instance(flavor)?.versions().await
    }

    pub async fn resolve_instance(
        &self,
        flavor: &str,
        version: &str,
        loader_version: Option<String>,
    ) -> Result<InstanceProfile> {
        tracing::info!(
            flavor,
            version,
            ?loader_version,
            "resolving instance profile"
        );
        self.instance(flavor)?
            .resolve(&ResolveRequest {
                version: version.to_string(),
                loader_version,
            })
            .await
    }

    fn server(&self, flavor: &str) -> Result<&dyn ServerProvider> {
        self.servers
            .iter()
            .map(AsRef::as_ref)
            .find(|p| p.id() == flavor)
            .with_context(|| format!("unknown server flavor: {flavor}"))
    }

    fn instance(&self, flavor: &str) -> Result<&dyn InstanceProvider> {
        self.instances
            .iter()
            .map(AsRef::as_ref)
            .find(|p| p.id() == flavor)
            .with_context(|| format!("unknown instance flavor: {flavor}"))
    }
}

fn flavor(id: &str, name: &str) -> Flavor {
    Flavor {
        id: id.to_string(),
        name: name.to_string(),
    }
}
