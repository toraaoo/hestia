//! The typed settings store. The schema is one struct, `Settings`; a setting is a
//! field with its default, persisted as JSON. Internal code reads a `settings()`
//! snapshot and writes through `update()`; the dotted-path get/set serve the
//! `config.*` channels and reject unknown keys and type-mismatched values.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use proto::naming;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::minecraft::launch::{normalize_memory, parse_jvm_args, JavaSettings};

/// The config schema. A setting is a typed field with its default; a nested
/// struct becomes a sub-object. The reserved keys (home, autostart) are routed
/// by the daemon's config service, not stored here.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// JVM defaults applied to any server or instance whose record leaves the
    /// matching per-entry setting unset.
    pub defaults: JvmDefaults,
    /// Credentials for the content sources that need one.
    pub content: ContentSettings,
    /// The news and notices feed.
    pub announcements: AnnounceSettings,
    /// What an instance is allowed to do beyond the safe default.
    pub instance: InstanceSettings,
    /// Shared settings/configs across instances.
    pub sync: SyncSettings,
    /// Corrections applied over a modpack's own declarations.
    pub modpack: ModpackSettings,
}

/// The corrections over what a modpack claims about itself, addressed by the
/// kebab-case keys `modpack.default-excludes`, `modpack.exclude-files`,
/// `modpack.force-include-files` and `modpack.overrides-exclusions`. The three
/// list keys take itzg's own delimiters (comma or newline, `#` comments), so a
/// docker-mc-server user's `MODRINTH_EXCLUDE_FILES` pastes in unchanged.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ModpackSettings {
    /// Whether the shipped client-mod exclude table applies to server installs.
    pub default_excludes: bool,
    pub exclude_files: String,
    pub force_include_files: String,
    pub overrides_exclusions: String,
}

impl Default for ModpackSettings {
    fn default() -> Self {
        ModpackSettings {
            default_excludes: true,
            exclude_files: String::new(),
            force_include_files: String::new(),
            overrides_exclusions: String::new(),
        }
    }
}

/// What an instance may do beyond one session at a time, keyed
/// `instance.multi-session`. Concurrent sessions share a single `data/` — the
/// worlds, the configs, the content mirror — which Minecraft arbitrates only
/// per world, so the capability is off until a user asks for it.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceSettings {
    pub multi_session: bool,
}

/// Whether instances share their settings targets at all. Sync moves a user's
/// own files into a common store, so it is switchable: off, a launch reconciles
/// nothing and every instance keeps what it has — links already made stay, since
/// hestia never breaks one behind the user's back.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct SyncSettings {
    pub enabled: bool,
}

impl Default for SyncSettings {
    fn default() -> Self {
        SyncSettings { enabled: true }
    }
}

/// Whether the launcher fetches its announcement feed. This is the daemon's
/// only *unprompted* outbound request — the update check runs on demand — so
/// it is switchable rather than assumed.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct AnnounceSettings {
    pub enabled: bool,
}

impl Default for AnnounceSettings {
    fn default() -> Self {
        AnnounceSettings { enabled: true }
    }
}

/// The launcher-wide JVM defaults, addressed by the kebab-case config keys
/// `defaults.memory` / `defaults.jvm-args`. Serializes camelCase like every
/// other struct (`jvmArgs` on disk); the dotted-path get/set translates the
/// kebab key to the camel field via `naming::config_key_to_field`. Plain
/// strings so both keys always resolve.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct JvmDefaults {
    pub memory: String,
    pub jvm_args: String,
}

/// Per-source credentials, keyed `content.curseforge-key`. CurseForge is
/// offered only once a key resolves — this, else the build-time one.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ContentSettings {
    pub curseforge_key: String,
}

impl Settings {
    /// Validate and canonicalise after a raw dotted-path set — the same rules
    /// the per-entry `memory`/`jvm-args` keys enforce.
    fn normalize(&mut self) -> Result<(), String> {
        if !self.defaults.memory.trim().is_empty() {
            self.defaults.memory =
                normalize_memory(&self.defaults.memory).map_err(|e| e.to_string())?;
        } else {
            self.defaults.memory = String::new();
        }
        self.defaults.jvm_args = parse_jvm_args(&self.defaults.jvm_args)
            .map_err(|e| e.to_string())?
            .join(" ");
        self.content.curseforge_key = self.content.curseforge_key.trim().to_string();
        Ok(())
    }

    /// The JVM defaults as launch settings, for `JavaSettings::or_defaults`.
    pub fn java_defaults(&self) -> JavaSettings {
        JavaSettings {
            memory: (!self.defaults.memory.is_empty()).then(|| self.defaults.memory.clone()),
            jvm_args: self
                .defaults
                .jvm_args
                .split_whitespace()
                .map(str::to_string)
                .collect(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("unknown config key: {0}")]
    UnknownKey(String),
    #[error("{0}")]
    TypeMismatch(String),
    #[error("invalid value for {key}: {source}")]
    InvalidValue {
        key: String,
        source: serde_json::Error,
    },
    #[error("invalid value for {key}: {message}")]
    Rejected { key: String, message: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct Config {
    inner: Mutex<Inner>,
}

struct Inner {
    path: PathBuf,
    settings: Settings,
}

impl Config {
    pub fn new(path: PathBuf) -> Self {
        let settings = load_settings(&path);
        Config {
            inner: Mutex::new(Inner { path, settings }),
        }
    }

    pub fn settings(&self) -> Settings {
        self.inner.lock().unwrap().settings.clone()
    }

    pub fn update(&self, mutate: impl FnOnce(&mut Settings)) -> Result<(), ConfigError> {
        let mut inner = self.inner.lock().unwrap();
        mutate(&mut inner.settings);
        save_settings(&inner.path, &inner.settings)
    }

    /// Return the value at a dotted key path, or `UnknownKey` if absent.
    pub fn get(&self, key: &str) -> Result<Value, ConfigError> {
        let inner = self.inner.lock().unwrap();
        let doc = serde_json::to_value(&inner.settings).unwrap_or(Value::Null);
        find_node(&doc, key)
            .cloned()
            .ok_or_else(|| ConfigError::UnknownKey(key.to_string()))
    }

    /// Set the value at a dotted key path, rejecting unknown keys and values of a
    /// different JSON kind than the existing setting.
    pub fn set(&self, key: &str, value: Value) -> Result<(), ConfigError> {
        let mut inner = self.inner.lock().unwrap();
        let mut doc = serde_json::to_value(&inner.settings).unwrap_or(Value::Null);
        {
            let node = find_node_mut(&mut doc, key)
                .ok_or_else(|| ConfigError::UnknownKey(key.to_string()))?;
            if !same_json_kind(node, &value) {
                return Err(ConfigError::TypeMismatch(format!(
                    "{key} expects a {}",
                    kind_name(node)
                )));
            }
            *node = value;
        }
        let mut settings: Settings =
            serde_json::from_value(doc).map_err(|source| ConfigError::InvalidValue {
                key: key.to_string(),
                source,
            })?;
        settings
            .normalize()
            .map_err(|message| ConfigError::Rejected {
                key: key.to_string(),
                message,
            })?;
        inner.settings = settings;
        save_settings(&inner.path, &inner.settings)?;
        tracing::info!(key, "config updated");
        Ok(())
    }

    /// The effective settings as a JSON object, keyed by the kebab-case
    /// `config.*` vocabulary the user sets (`defaults.jvm-args`), not the
    /// camelCase serialized fields.
    pub fn all(&self) -> Value {
        let inner = self.inner.lock().unwrap();
        let settings = serde_json::to_value(&inner.settings)
            .unwrap_or_else(|_| Value::Object(Default::default()));
        naming::settings_to_config_keys(settings)
    }

    pub fn reload(&self, path: PathBuf) {
        let mut inner = self.inner.lock().unwrap();
        inner.settings = load_settings(&path);
        tracing::debug!(path = %path.display(), "config store reloaded");
        inner.path = path;
    }
}

fn load_settings(path: &Path) -> Settings {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_settings(path: &Path, settings: &Settings) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(settings).expect("Settings serializes");
    std::fs::write(path, format!("{text}\n"))?;
    Ok(())
}

// The config keys are kebab-case (`defaults.jvm-args`); the settings serialize
// camelCase, so each segment is translated to its field name before lookup.
fn find_node<'a>(root: &'a Value, key: &str) -> Option<&'a Value> {
    let mut node = root;
    for segment in key.split('.') {
        if segment.is_empty() {
            return None;
        }
        node = node
            .as_object()?
            .get(&naming::config_key_to_field(segment))?;
    }
    Some(node)
}

fn find_node_mut<'a>(root: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    let mut node = root;
    for segment in key.split('.') {
        if segment.is_empty() {
            return None;
        }
        node = node
            .as_object_mut()?
            .get_mut(&naming::config_key_to_field(segment))?;
    }
    Some(node)
}

fn same_json_kind(a: &Value, b: &Value) -> bool {
    if a.is_number() && b.is_number() {
        return true;
    }
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

fn kind_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
