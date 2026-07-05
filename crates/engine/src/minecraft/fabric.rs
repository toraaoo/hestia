//! Fabric — the loader profile layered over a base game version. The server is a
//! self-contained launcher jar; the client merges Fabric's libraries and main
//! class over the vanilla profile.

use anyhow::Result;
use async_trait::async_trait;
use proto::minecraft::{Artifact, GameVersion, InstanceProfile, ServerProfile, VersionKind};

use super::meta::{fabric, mojang};
use super::provider::{InstanceProvider, ResolveRequest, ServerProvider};

const ID: &str = "fabric";
const NAME: &str = "Fabric";

async fn game_versions() -> Result<Vec<GameVersion>> {
    Ok(fabric::game_versions()
        .await?
        .into_iter()
        .map(|(id, stable)| GameVersion {
            id,
            kind: if stable {
                VersionKind::Release
            } else {
                VersionKind::Snapshot
            },
            stable,
        })
        .collect())
}

async fn resolve_loader(request: &ResolveRequest) -> Result<String> {
    match &request.loader_version {
        Some(loader) => Ok(loader.clone()),
        None => fabric::latest_loader(&request.version).await,
    }
}

pub struct FabricServer;

#[async_trait]
impl ServerProvider for FabricServer {
    fn id(&self) -> &'static str {
        ID
    }
    fn name(&self) -> &'static str {
        NAME
    }

    async fn versions(&self) -> Result<Vec<GameVersion>> {
        game_versions().await
    }

    async fn resolve(&self, request: &ResolveRequest) -> Result<ServerProfile> {
        let loader = resolve_loader(request).await?;
        let installer = fabric::latest_installer().await?;
        let base = mojang::version_json(&request.version).await?;
        let url = fabric::server_launcher_url(&request.version, &loader, &installer);
        Ok(ServerProfile {
            flavor: ID.to_string(),
            game_version: request.version.clone(),
            loader_version: Some(loader.clone()),
            primary: Artifact {
                filename: format!(
                    "fabric-server-mc.{}-loader.{}-launcher.{}.jar",
                    request.version, loader, installer
                ),
                url,
                size: 0,
                checksum: None,
            },
            libraries: Vec::new(),
            java_major: mojang::java_major(&base),
            main_class: String::new(),
        })
    }
}

pub struct FabricInstance;

#[async_trait]
impl InstanceProvider for FabricInstance {
    fn id(&self) -> &'static str {
        ID
    }
    fn name(&self) -> &'static str {
        NAME
    }

    async fn versions(&self) -> Result<Vec<GameVersion>> {
        game_versions().await
    }

    async fn resolve(&self, request: &ResolveRequest) -> Result<InstanceProfile> {
        let loader = resolve_loader(request).await?;
        let base = mojang::version_json(&request.version).await?;
        let profile = fabric::profile_json(&request.version, &loader).await?;

        let mut libraries = mojang::libraries(&base);
        libraries.extend(fabric::libraries(&profile));

        let mut jvm_args = mojang::jvm_args(&base);
        jvm_args.extend(fabric::jvm_args(&profile));

        Ok(InstanceProfile {
            flavor: ID.to_string(),
            game_version: request.version.clone(),
            loader_version: Some(loader),
            client: mojang::client_artifact(&base)?,
            libraries,
            asset_index: mojang::asset_index(&base)?,
            java_major: mojang::java_major(&base),
            main_class: fabric::client_main_class(&profile),
            jvm_args,
            game_args: mojang::game_args(&base),
        })
    }
}
