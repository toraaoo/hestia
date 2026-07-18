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
use crate::skins::{defaults, mojang, validate_skin_png};

impl Engine {
    /// A live token for `account` (name or uuid; empty = the default account).
    async fn skin_token(&self, account: &str) -> Result<String> {
        let reference = if account.trim().is_empty() {
            self.accounts()
                .default_account()
                .map(|a| a.uuid)
                .context("no account is signed in")?
        } else {
            account.trim().to_string()
        };
        self.accounts().access_token(&reference).await
    }

    /// The account's skin picture: library entries, the vanilla defaults, and —
    /// when neither covers it — the equipped external skin; plus the owned
    /// capes. At most one skin and one cape are marked equipped.
    pub async fn list_skins(&self, account: &str) -> Result<(Vec<Skin>, Vec<Cape>)> {
        let token = self.skin_token(account).await?;
        let profile = mojang::fetch_profile(&token).await?;
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

        let token = self.skin_token(account).await?;
        let before = mojang::fetch_profile(&token).await?;
        self.preserve_current_skin(&before).await;

        let after = match mojang::upload_skin(&token, png.clone(), variant).await? {
            Some(profile) => profile,
            None => mojang::fetch_profile(&token).await?,
        };
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

    /// Rewrite a library entry's label and variant. When the edited skin is
    /// the one equipped and its variant changed, the texture is re-uploaded
    /// under the new variant — otherwise `list_skins` would sync the local
    /// variant back from the profile and silently undo the edit.
    pub async fn update_skin(
        &self,
        account: &str,
        key: &str,
        name: &str,
        variant: SkinVariant,
    ) -> Result<Skin> {
        let previous = self
            .skins()
            .entry(key)
            .with_context(|| format!("no library skin matches '{key}'"))?;
        let entry = self
            .skins()
            .update(key, name, variant)?
            .with_context(|| format!("no library skin matches '{key}'"))?;

        let mut equipped = false;
        if previous.variant != variant {
            let token = self.skin_token(account).await?;
            let profile = mojang::fetch_profile(&token).await?;
            if profile.active_skin().is_some_and(|a| a.key == key) {
                equipped = true;
                let png = self.skins().texture(key)?;
                mojang::upload_skin(&token, png, variant).await?;
                tracing::info!(
                    key,
                    ?variant,
                    "re-equipped the edited skin under its new variant"
                );
            }
        }

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

    /// Equip a library or default skin by its key from `skin.list`.
    pub async fn equip_skin(&self, account: &str, key: &str) -> Result<()> {
        let token = self.skin_token(account).await?;
        let before = mojang::fetch_profile(&token).await?;
        self.preserve_current_skin(&before).await;

        if let Some(entry) = self.skins().entry(key) {
            let png = self.skins().texture(key)?;
            let after = match mojang::upload_skin(&token, png, entry.variant).await? {
                Some(profile) => profile,
                None => mojang::fetch_profile(&token).await?,
            };
            if let Some(active) = after.active_skin() {
                self.skins().rekey(key, &active.key)?;
            }
        } else if let Some(default) = defaults::find(key) {
            mojang::set_skin_url(&token, &defaults::texture_url(key), default.variant).await?;
        } else {
            bail!("no skin matches '{key}'");
        }
        tracing::info!(key, "skin equipped");
        Ok(())
    }

    /// Reset the account to its uuid-derived default skin.
    pub async fn reset_skin(&self, account: &str) -> Result<()> {
        let token = self.skin_token(account).await?;
        let before = mojang::fetch_profile(&token).await?;
        self.preserve_current_skin(&before).await;
        mojang::reset_skin(&token).await?;
        tracing::info!("skin reset to the default");
        Ok(())
    }

    pub async fn equip_cape(&self, account: &str, cape_id: &str) -> Result<()> {
        let token = self.skin_token(account).await?;
        mojang::set_cape(&token, cape_id).await?;
        tracing::info!(cape = %cape_id, "cape equipped");
        Ok(())
    }

    pub async fn clear_cape(&self, account: &str) -> Result<()> {
        let token = self.skin_token(account).await?;
        mojang::clear_cape(&token).await?;
        tracing::info!("cape cleared");
        Ok(())
    }

    /// Save the equipped texture into the library when neither the library nor
    /// the defaults already record it. Best-effort: a failure must not block
    /// the change the user asked for.
    async fn preserve_current_skin(&self, profile: &mojang::Profile) {
        let Some(active) = profile.active_skin() else {
            return;
        };
        if defaults::find(&active.key).is_some() || self.skins().entry(&active.key).is_some() {
            return;
        }
        let saved = match mojang::fetch_texture(&active.url).await {
            Ok(png) => self
                .skins()
                .add_keyed(&active.key, &png, active.variant, ""),
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

fn data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(png))
}
