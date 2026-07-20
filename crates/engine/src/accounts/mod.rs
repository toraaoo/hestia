//! Minecraft accounts signed in through Microsoft, persisted with their tokens
//! in `<data_home>/accounts.json` (owner-only on POSIX). Both sign-in methods
//! converge on the shared signed tail; tokens never leave the daemon.

mod microsoft;
mod signing;

pub(crate) use microsoft::USER_AGENT;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Result};
use proto::accounts::{Account, LoginMethod};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use microsoft::{
    launcher_login, minecraft_profile, poll_device_code, redeem_code, refresh_oauth,
    request_device_code, request_device_token, sisu_authenticate, sisu_authorize, xsts_authorize,
    OAuthTokens,
};
use signing::{base64url_nopad, format_uuid_v4, hex, random_bytes, ProofKey};

const REFRESH_MARGIN_SECONDS: i64 = 300;

/// What the user must act on to finish a sign-in.
pub struct LoginChallenge {
    pub id: String,
    pub method: LoginMethod,
    pub url: String,
    pub user_code: String,
    pub verification_uri: String,
}

struct LoginSession {
    method: LoginMethod,
    key: Option<ProofKey>,
    device_token: String,
    verifier: String,
    session_id: String,
    device_code: String,
    interval_seconds: i64,
    expires_at: i64,
    clock_offset: i64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct StoredAccount {
    uuid: String,
    name: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    expires_at: i64,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AccountsFile {
    #[serde(default)]
    accounts: Vec<StoredAccount>,
    /// The account launches use when none is named; empty falls back to the
    /// first signed-in one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    default_uuid: String,
}

pub struct Accounts {
    inner: Mutex<Inner>,
}

struct Inner {
    path: PathBuf,
    pending: HashMap<String, LoginSession>,
}

impl Accounts {
    pub fn new(path: PathBuf) -> Self {
        Accounts {
            inner: Mutex::new(Inner {
                path,
                pending: HashMap::new(),
            }),
        }
    }

    pub fn reload(&self, path: PathBuf) {
        self.inner.lock().unwrap().path = path;
    }

    fn path(&self) -> PathBuf {
        self.inner.lock().unwrap().path.clone()
    }

    pub fn list(&self) -> Vec<Account> {
        load(&self.path())
            .accounts
            .into_iter()
            .map(|a| Account {
                uuid: a.uuid,
                name: a.name,
            })
            .collect()
    }

    pub fn has_account(&self) -> bool {
        !load(&self.path()).accounts.is_empty()
    }

    /// The account launches use when none is named: the switched-to one, else
    /// the first signed-in one.
    pub fn default_account(&self) -> Option<Account> {
        let file = load(&self.path());
        let stored = file
            .accounts
            .iter()
            .find(|a| a.uuid == file.default_uuid)
            .or_else(|| file.accounts.first())?;
        Some(Account {
            uuid: stored.uuid.clone(),
            name: stored.name.clone(),
        })
    }

    /// Make `reference` (name or uuid) the default account. Returns `None` when
    /// no account matches.
    pub fn switch(&self, reference: &str) -> Result<Option<Account>> {
        let path = self.path();
        let mut file = load(&path);
        let Some(chosen) = file
            .accounts
            .iter()
            .find(|a| a.uuid == reference || a.name == reference)
            .map(|a| Account {
                uuid: a.uuid.clone(),
                name: a.name.clone(),
            })
        else {
            return Ok(None);
        };
        file.default_uuid = chosen.uuid.clone();
        save(&path, &file)?;
        tracing::info!(name = %chosen.name, "switched default account");
        Ok(Some(chosen))
    }

    pub async fn begin_login(&self, method: LoginMethod) -> Result<LoginChallenge> {
        let id = format_uuid_v4(&random_bytes(16));
        tracing::info!(?method, "starting sign-in");

        if method == LoginMethod::DeviceCode {
            let device = request_device_code().await?;
            let session = LoginSession {
                method,
                key: None,
                device_token: String::new(),
                verifier: String::new(),
                session_id: String::new(),
                device_code: device.device_code,
                interval_seconds: device.interval_seconds,
                expires_at: now_seconds() + device.expires_in_seconds,
                clock_offset: 0,
            };
            self.inner
                .lock()
                .unwrap()
                .pending
                .insert(id.clone(), session);
            return Ok(LoginChallenge {
                id,
                method,
                url: String::new(),
                user_code: device.user_code,
                verification_uri: device.verification_uri,
            });
        }

        let key = ProofKey::generate();
        let device = request_device_token(&key).await?;
        let verifier = hex(&random_bytes(64));
        let challenge = base64url_nopad(&sha256_bytes(&verifier));
        let state = hex(&random_bytes(16));
        let auth =
            sisu_authenticate(&device.token, &challenge, &state, &key, device.clock_offset).await?;

        let session = LoginSession {
            method,
            key: Some(key),
            device_token: device.token,
            verifier,
            session_id: auth.session_id,
            device_code: String::new(),
            interval_seconds: 5,
            expires_at: 0,
            clock_offset: device.clock_offset,
        };
        self.inner
            .lock()
            .unwrap()
            .pending
            .insert(id.clone(), session);
        Ok(LoginChallenge {
            id,
            method,
            url: auth.url,
            user_code: String::new(),
            verification_uri: String::new(),
        })
    }

    pub async fn complete_login(&self, id: &str, code: &str) -> Result<Account> {
        let session = {
            let mut inner = self.inner.lock().unwrap();
            inner
                .pending
                .remove(id)
                .ok_or_else(|| anyhow!("no sign-in is in progress for this request"))?
        };

        let (oauth, xsts) = if session.method == LoginMethod::DeviceCode {
            let oauth = await_device_tokens(&session).await?;
            let key = ProofKey::generate();
            let device = request_device_token(&key).await?;
            let authorization = sisu_authorize(
                "",
                &oauth.access_token,
                &device.token,
                &key,
                device.clock_offset,
            )
            .await?;
            let xsts =
                xsts_authorize(&authorization, &device.token, &key, device.clock_offset).await?;
            (oauth, xsts)
        } else {
            let oauth = redeem_code(code, &session.verifier).await?;
            let key = session
                .key
                .as_ref()
                .ok_or_else(|| anyhow!("sisu session lost its proof key"))?;
            let authorization = sisu_authorize(
                &session.session_id,
                &oauth.access_token,
                &session.device_token,
                key,
                session.clock_offset,
            )
            .await?;
            let xsts = xsts_authorize(
                &authorization,
                &session.device_token,
                key,
                session.clock_offset,
            )
            .await?;
            (oauth, xsts)
        };

        let minecraft_token = launcher_login(&xsts).await?;
        let profile = minecraft_profile(&minecraft_token).await?;

        let record = StoredAccount {
            uuid: profile.uuid.clone(),
            name: profile.name.clone(),
            refresh_token: oauth.refresh_token,
            access_token: minecraft_token,
            expires_at: now_seconds() + oauth.expires_in,
        };

        {
            let path = self.path();
            let mut file = load(&path);
            file.accounts.retain(|a| a.uuid != record.uuid);
            file.accounts.push(record);
            save(&path, &file)?;
        }
        tracing::info!(name = %profile.name, uuid = %profile.uuid, "signed in");
        Ok(Account {
            uuid: profile.uuid,
            name: profile.name,
        })
    }

    /// A currently-valid Minecraft access token for `reference` (uuid or name),
    /// rotating the stored tokens through Microsoft when they are at or near
    /// expiry.
    pub async fn access_token(&self, reference: &str) -> Result<String> {
        let path = self.path();
        let mut account = {
            let file = load(&path);
            file.accounts
                .into_iter()
                .find(|a| a.uuid == reference || a.name == reference)
                .ok_or_else(|| anyhow!("no account matches '{reference}'"))?
        };

        if account.expires_at - now_seconds() > REFRESH_MARGIN_SECONDS {
            return Ok(account.access_token);
        }

        rotate_tokens(&mut account).await?;
        let mut file = load(&path);
        if let Some(existing) = file.accounts.iter_mut().find(|a| a.uuid == account.uuid) {
            *existing = account.clone();
        } else {
            file.accounts.push(account.clone());
        }
        save(&path, &file)?;
        Ok(account.access_token)
    }

    pub fn remove(&self, reference: &str) -> Result<bool> {
        let path = self.path();
        let mut file = load(&path);
        let before = file.accounts.len();
        file.accounts
            .retain(|a| a.uuid != reference && a.name != reference);
        if file.accounts.len() == before {
            return Ok(false);
        }
        if !file.accounts.iter().any(|a| a.uuid == file.default_uuid) {
            file.default_uuid = String::new();
        }
        save(&path, &file)?;
        tracing::info!(reference, "signed out account");
        Ok(true)
    }
}

async fn await_device_tokens(session: &LoginSession) -> Result<OAuthTokens> {
    let interval = Duration::from_secs(session.interval_seconds.max(1) as u64);
    while now_seconds() < session.expires_at {
        if let Some(tokens) = poll_device_code(&session.device_code).await? {
            return Ok(tokens);
        }
        tokio::time::sleep(interval).await;
    }
    bail!("the sign-in request expired before it was approved; run 'hestia account login' again")
}

async fn rotate_tokens(account: &mut StoredAccount) -> Result<()> {
    if account.refresh_token.is_empty() {
        bail!("this account has no refresh token; sign in again");
    }
    tracing::debug!(uuid = %account.uuid, "refreshing minecraft token");
    let oauth = refresh_oauth(&account.refresh_token).await?;
    let key = ProofKey::generate();
    let device = request_device_token(&key).await?;
    let authorization = sisu_authorize(
        "",
        &oauth.access_token,
        &device.token,
        &key,
        device.clock_offset,
    )
    .await?;
    let xsts = xsts_authorize(&authorization, &device.token, &key, device.clock_offset).await?;

    account.access_token = launcher_login(&xsts).await?;
    if !oauth.refresh_token.is_empty() {
        account.refresh_token = oauth.refresh_token;
    }
    account.expires_at = now_seconds() + oauth.expires_in;
    Ok(())
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn sha256_bytes(text: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.finalize().to_vec()
}

fn load(path: &Path) -> AccountsFile {
    let Ok(text) = std::fs::read_to_string(path) else {
        return AccountsFile::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save(path: &Path, file: &AccountsFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(file).expect("accounts file serializes");
    std::fs::write(path, format!("{text}\n"))?;
    // The file holds tokens: keep it owner-only where permissions exist.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}
