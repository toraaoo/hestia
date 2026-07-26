//! Skin and cape flows: the account's Mojang profile reconciled with the local
//! skin library and the vanilla defaults. Every change first preserves the
//! currently equipped texture into the library when nothing else records it —
//! switching away from an externally-set skin must not lose it (Modrinth's
//! rule, kept here).

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use proto::skins::{Cape, Skin, SkinSource, SkinVariant};

use super::Engine;
use crate::skins::{defaults, mojang, validate_skin_png, LibraryEntry};

impl Engine {
    fn skin_reference(&self, account: &str) -> Result<String> {
        if account.trim().is_empty() {
            self.accounts()
                .default_account()
                .map(|a| a.uuid)
                .context("no account is signed in")
        } else {
            Ok(account.trim().to_string())
        }
    }

    async fn skin_session(&self, account: &str) -> Result<(String, String)> {
        let reference = self.skin_reference(account)?;
        let token = self.accounts().access_token(&reference).await?;
        Ok((reference, token))
    }

    /// The account's skin picture: library entries, the vanilla defaults, and —
    /// when neither covers it — the equipped external skin; plus the owned
    /// capes. At most one skin and one cape are marked equipped.
    pub async fn list_skins(&self, account: &str) -> Result<(Vec<Skin>, Vec<Cape>)> {
        let (reference, token) = self.skin_session(account).await?;
        let profile = match self.skins().cached_profile(&reference) {
            Some(profile) => profile,
            None => {
                let profile = mojang::fetch_profile(&token).await?;
                self.skins().store_profile(&reference, profile.clone());
                profile
            }
        };
        let active = profile.active_skin();

        let mut skins = Vec::new();
        let mut equipped_seen = false;
        for entry in self.skins().list() {
            // A library row holding a default's texture is redundant: the
            // default card below represents it.
            if defaults::find(&entry.key).is_some() {
                continue;
            }
            let mut variant = entry.variant;
            let equipped = !equipped_seen && active.is_some_and(|a| a.key == entry.key);
            if equipped {
                let profile_variant = active.expect("equipped implies active").variant;
                if profile_variant != variant {
                    self.skins().sync_variant(&entry.key, profile_variant)?;
                    variant = profile_variant;
                }
            }
            equipped_seen |= equipped;
            let texture = match self.skins().texture(&entry.key) {
                Ok(png) => data_url(&png),
                Err(e) => {
                    tracing::warn!(key = %entry.key, error = %e, "skipping an unreadable library skin");
                    continue;
                }
            };
            skins.push(Skin {
                key: entry.key,
                name: entry.name,
                variant,
                texture,
                source: SkinSource::Library,
                equipped,
            });
        }

        for default in defaults::DEFAULT_SKINS {
            let equipped = !equipped_seen && active.is_some_and(|a| a.key == default.key);
            equipped_seen |= equipped;
            skins.push(Skin {
                key: default.key.to_string(),
                name: default.name.to_string(),
                variant: default.variant,
                texture: defaults::texture_url(default.key),
                source: SkinSource::Default,
                equipped,
            });
        }

        if !equipped_seen {
            if let Some(active) = active {
                skins.push(Skin {
                    key: active.key.clone(),
                    name: String::new(),
                    variant: active.variant,
                    texture: active.url.clone(),
                    source: SkinSource::External,
                    equipped: true,
                });
            }
        }

        let capes = profile
            .capes
            .iter()
            .map(|cape| Cape {
                id: cape.id.clone(),
                name: cape.name.clone(),
                texture: cape.url.clone(),
                equipped: cape.active,
            })
            .collect();
        Ok((skins, capes))
    }

    /// Upload a new skin (base64 PNG), equip it, and save it to the library
    /// under the texture key Mojang mints for it.
    pub async fn add_skin(
        &self,
        account: &str,
        name: &str,
        variant: SkinVariant,
        data: &str,
    ) -> Result<Skin> {
        let png = STANDARD
            .decode(data.trim())
            .context("the skin data is not valid base64")?;
        validate_skin_png(&png)?;

        let (reference, token) = self.skin_session(account).await?;
        let before = mojang::fetch_profile(&token).await?;
        self.preserve_current_skin(&before).await;

        let after = match mojang::upload_skin(&token, png.clone(), variant).await? {
            Some(profile) => profile,
            None => mojang::fetch_profile(&token).await?,
        };
        self.skins().store_profile(&reference, after.clone());
        let key = after
            .active_skin()
            .map(|s| s.key.clone())
            .context("Mojang accepted the skin but reports none equipped")?;
        let entry = self.skins().add_keyed(&key, &png, variant, name)?;
        tracing::info!(key = %entry.key, "skin uploaded and equipped");
        Ok(Skin {
            key: entry.key,
            name: entry.name,
            variant,
            texture: data_url(&png),
            source: SkinSource::Library,
            equipped: true,
        })
    }

    /// Rewrite a library entry's label and variant. An equipped skin not yet in
    /// the library is adopted into it first. The variant re-upload is required:
    /// otherwise `list_skins` syncs the local variant back from the profile and
    /// silently undoes the edit.
    pub async fn update_skin(
        &self,
        account: &str,
        key: &str,
        name: &str,
        variant: SkinVariant,
    ) -> Result<Skin> {
        let previous = match self.skins().entry(key) {
            Some(entry) => entry,
            None => self.adopt_equipped_skin(account, key).await?,
        };
        let entry = self.skins().update(key, name, variant)?.ok_or_else(|| {
            proto::error::ErrorInfo::SkinNotFound {
                key: key.to_string(),
            }
        })?;

        if previous.variant != variant {
            let (reference, token) = self.skin_session(account).await?;
            let profile = mojang::fetch_profile(&token).await?;
            if profile.active_skin().is_some_and(|a| a.key == key) {
                let png = self.skins().texture(key)?;
                match mojang::upload_skin(&token, png, variant).await? {
                    Some(profile) => self.skins().store_profile(&reference, profile),
                    None => self.skins().invalidate_profile(&reference),
                }
                tracing::info!(
                    key,
                    ?variant,
                    "re-equipped the edited skin under its new variant"
                );
            }
        }

        let equipped = self.skin_equipped(account, key);
        let texture = data_url(&self.skins().texture(key)?);
        Ok(Skin {
            key: entry.key,
            name: entry.name,
            variant,
            texture,
            source: SkinSource::Library,
            equipped,
        })
    }

    async fn adopt_equipped_skin(&self, account: &str, key: &str) -> Result<LibraryEntry> {
        let (reference, token) = self.skin_session(account).await?;
        let profile = match self.skins().cached_profile(&reference) {
            Some(profile) => profile,
            None => {
                let profile = mojang::fetch_profile(&token).await?;
                self.skins().store_profile(&reference, profile.clone());
                profile
            }
        };
        let active = profile
            .active_skin()
            .filter(|a| a.key == key)
            .ok_or_else(|| proto::error::ErrorInfo::SkinNotFound {
                key: key.to_string(),
            })?;
        let png = mojang::fetch_texture(&active.url).await?;
        let entry = self.skins().add_keyed(key, &png, active.variant, "")?;
        tracing::info!(key, "adopted the equipped skin into the library");
        Ok(entry)
    }

    fn skin_equipped(&self, account: &str, key: &str) -> bool {
        let Ok(reference) = self.skin_reference(account) else {
            return false;
        };
        self.skins()
            .cached_profile(&reference)
            .and_then(|profile| profile.active_skin().map(|a| a.key == key))
            .unwrap_or(false)
    }

    /// Equip a library or default skin by its key from `skin.list`.
    pub async fn equip_skin(&self, account: &str, key: &str) -> Result<()> {
        let (reference, token) = self.skin_session(account).await?;
        let before = mojang::fetch_profile(&token).await?;
        self.preserve_current_skin(&before).await;

        if let Some(entry) = self.skins().entry(key) {
            let png = self.skins().texture(key)?;
            let after = match mojang::upload_skin(&token, png, entry.variant).await? {
                Some(profile) => profile,
                None => mojang::fetch_profile(&token).await?,
            };
            self.skins().store_profile(&reference, after.clone());
            if let Some(active) = after.active_skin() {
                self.skins().rekey(key, &active.key)?;
            }
        } else if let Some(default) = defaults::find(key) {
            match mojang::set_skin_url(&token, &defaults::texture_url(key), default.variant).await?
            {
                Some(profile) => self.skins().store_profile(&reference, profile),
                None => self.skins().invalidate_profile(&reference),
            }
        } else {
            bail!(proto::error::ErrorInfo::SkinNotFound {
                key: key.to_string()
            });
        }
        tracing::info!(key, "skin equipped");
        Ok(())
    }

    /// Reset the account to its uuid-derived default skin.
    pub async fn reset_skin(&self, account: &str) -> Result<()> {
        let (reference, token) = self.skin_session(account).await?;
        let before = mojang::fetch_profile(&token).await?;
        self.preserve_current_skin(&before).await;
        mojang::reset_skin(&token).await?;
        self.skins().invalidate_profile(&reference);
        tracing::info!("skin reset to the default");
        Ok(())
    }

    pub async fn equip_cape(&self, account: &str, cape_id: &str) -> Result<()> {
        let (reference, token) = self.skin_session(account).await?;
        mojang::set_cape(&token, cape_id).await?;
        self.skins().invalidate_profile(&reference);
        tracing::info!(cape = %cape_id, "cape equipped");
        Ok(())
    }

    pub async fn clear_cape(&self, account: &str) -> Result<()> {
        let (reference, token) = self.skin_session(account).await?;
        mojang::clear_cape(&token).await?;
        self.skins().invalidate_profile(&reference);
        tracing::info!("cape cleared");
        Ok(())
    }

    /// Save the equipped texture into the library when neither the library nor
    /// the defaults already record it, under an auto-generated "Skin N" name.
    /// Best-effort: a failure must not block the change the user asked for.
    async fn preserve_current_skin(&self, profile: &mojang::Profile) {
        let Some(active) = profile.active_skin() else {
            return;
        };
        if defaults::find(&active.key).is_some() || self.skins().entry(&active.key).is_some() {
            return;
        }
        let name = self.next_library_skin_name();
        let saved = match mojang::fetch_texture(&active.url).await {
            Ok(png) => self
                .skins()
                .add_keyed(&active.key, &png, active.variant, &name),
            Err(e) => Err(e),
        };
        match saved {
            Ok(_) => {
                tracing::info!(key = %active.key, "preserved the replaced skin in the library")
            }
            Err(e) => {
                tracing::warn!(key = %active.key, error = %e, "could not preserve the replaced skin")
            }
        }
    }
}

impl Engine {
    fn next_library_skin_name(&self) -> String {
        let highest = self
            .skins()
            .list()
            .iter()
            .filter_map(|e| {
                e.name
                    .strip_prefix("Skin ")
                    .and_then(|n| n.trim().parse::<u32>().ok())
            })
            .max()
            .unwrap_or(0);
        format!("Skin {}", highest + 1)
    }
}

fn data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(png))
}
