//! Bukkit and Spigot — the two server platforms SpigotMC publishes, and the
//! only flavors whose jar is compiled on the machine that runs it.
//!
//! Mojang's takedown means neither jar may be redistributed, so unlike every
//! other flavor there is nothing to fetch: the launch profile *names* a jar
//! that does not exist yet, carrying no URL, and the provider `install` hook
//! runs SpigotMC's BuildTools to produce it (see [`buildtools`]). That is the
//! seam NeoForge already established for a flavor whose jar has to be built.
//!
//! There is one build per game version, not a stream of them, so neither
//! flavor has a loader version to pin — the game version selects the build.
//! Neither has a client either, so neither implements `InstanceProvider`.

mod buildtools;

use anyhow::Result;
use async_trait::async_trait;
use proto::content::ContentKind;
use proto::minecraft::{Artifact, GameVersion, ServerProfile};

use self::buildtools::Product;
use super::materialize::OnProgress;
use super::meta::{mojang, spigot};
use super::provider::{InstallRequest, Loads, ResolveRequest, ServerProvider};
use super::REQUIRED_JAVA_MAJORS;

/// What the versions predating the hub's `javaVersions` field (the 1.8 line and
/// older) were built with.
const LEGACY_JAVA_MAJOR: i32 = 8;

/// The game versions BuildTools can build, ordered and typed by Mojang's
/// manifest.
///
/// Filtering against the manifest is not presentation here as it is for Paper —
/// it is the *selection*. The hub indexes its metadata by Jenkins build number
/// as well as by game version, so all but a few dozen of the four thousand
/// names it publishes are build numbers; a name Mojang does not list is one of
/// those, never a version to offer.
async fn game_versions() -> Result<Vec<GameVersion>> {
    let published = spigot::versions().await?;
    Ok(mojang::versions()
        .await?
        .into_iter()
        .filter(|v| published.iter().any(|p| p == &v.id))
        .collect())
}

/// The Java a version is built and run with. The hub states a range of
/// class-file majors; the low end is the version's real requirement, narrowed
/// to a runtime the launcher can actually install so a create fails at
/// resolution rather than at the Java step.
async fn java_major(version: &str) -> Result<i32> {
    let supported = spigot::java_versions(version).await?;
    let (Some(&low), Some(&high)) = (supported.first(), supported.last()) else {
        return Ok(LEGACY_JAVA_MAJOR);
    };
    Ok(REQUIRED_JAVA_MAJORS
        .into_iter()
        .find(|major| (low..=high).contains(major))
        .unwrap_or(low))
}

async fn resolve(product: Product, request: &ResolveRequest) -> Result<ServerProfile> {
    Ok(ServerProfile {
        flavor: product.flavor().to_string(),
        game_version: request.version.clone(),
        loader_version: None,
        // Named but not downloadable: an artifact with no URL is what tells
        // provisioning there is nothing to fetch, and the install hook writes
        // this file itself.
        primary: Artifact {
            filename: product.jar_name(&request.version),
            ..Artifact::default()
        },
        libraries: Vec::new(),
        java_major: java_major(&request.version).await?,
        main_class: String::new(),
        jvm_args: Vec::new(),
        args_file: String::new(),
    })
}

impl Product {
    fn flavor(self) -> &'static str {
        match self {
            Product::Bukkit => BUKKIT_ID,
            Product::Spigot => SPIGOT_ID,
        }
    }
}

/// Modrinth's own loader names, because the flavor id *is* the plugin loader a
/// content lookup filters by.
const BUKKIT_ID: &str = "bukkit";
const SPIGOT_ID: &str = "spigot";

pub struct BukkitServer;

#[async_trait]
impl ServerProvider for BukkitServer {
    fn id(&self) -> &'static str {
        BUKKIT_ID
    }
    fn name(&self) -> &'static str {
        "CraftBukkit"
    }
    fn summary(&self) -> &'static str {
        "The original plugin server, without Spigot's patches. Compiled here at create, which takes several minutes and needs git installed."
    }
    fn loads(&self) -> Loads {
        Some(ContentKind::Plugin)
    }

    async fn versions(&self) -> Result<Vec<GameVersion>> {
        game_versions().await
    }

    async fn resolve(&self, request: &ResolveRequest) -> Result<ServerProfile> {
        resolve(Product::Bukkit, request).await
    }

    async fn install(&self, request: &InstallRequest<'_>, on: OnProgress<'_>) -> Result<()> {
        buildtools::ensure(Product::Bukkit, request, on).await
    }
}

pub struct SpigotServer;

#[async_trait]
impl ServerProvider for SpigotServer {
    fn id(&self) -> &'static str {
        SPIGOT_ID
    }
    fn name(&self) -> &'static str {
        "Spigot"
    }
    fn summary(&self) -> &'static str {
        "Bukkit with performance patches, and the widest plugin ecosystem. Compiled here at create, which takes several minutes and needs git installed."
    }
    /// Spigot's plugins are filtered as `spigot`, never widened to `bukkit`:
    /// the two are separate Modrinth loaders, and a Spigot build carries API a
    /// plugin written against it may require.
    fn loads(&self) -> Loads {
        Some(ContentKind::Plugin)
    }

    async fn versions(&self) -> Result<Vec<GameVersion>> {
        game_versions().await
    }

    async fn resolve(&self, request: &ResolveRequest) -> Result<ServerProfile> {
        resolve(Product::Spigot, request).await
    }

    async fn install(&self, request: &InstallRequest<'_>, on: OnProgress<'_>) -> Result<()> {
        buildtools::ensure(Product::Spigot, request, on).await
    }
}
