//! The typed settings store. The schema is one struct, `Settings`; a setting is a
//! field with its default, persisted as JSON. Internal code reads a `settings()`
//! snapshot and writes through `update()`; the dotted-path get/set serve the
//! `config.*` channels and reject unknown keys and type-mismatched values.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::minecraft::launch::{normalize_memory, parse_jvm_args, JavaSettings};

/// The config schema. A setting is a typed field with its default; a nested
/// struct becomes a sub-object. The reserved keys (home, autostart) are routed
/// by the daemon's config service, not stored here.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct Settings {
    /// JVM defaults applied to any server or instance whose record leaves the
    /// matching per-entry setting unset.
    pub defaults: JvmDefaults,
}

/// The launcher-wide JVM defaults (`defaults.memory`, `defaults.jvm-args`);
/// empty means no default. Plain strings so the dotted-path get/set always
/// finds both keys.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct JvmDefaults {
    pub memory: String,
    pub jvm_args: String,
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

    /// The effective settings as a JSON object.
    pub fn all(&self) -> Value {
        let inner = self.inner.lock().unwrap();
        serde_json::to_value(&inner.settings).unwrap_or_else(|_| Value::Object(Default::default()))
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

fn find_node<'a>(root: &'a Value, key: &str) -> Option<&'a Value> {
    let mut node = root;
    for segment in key.split('.') {
        if segment.is_empty() {
            return None;
        }
        node = node.as_object()?.get(segment)?;
    }
    Some(node)
}

fn find_node_mut<'a>(root: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    let mut node = root;
    for segment in key.split('.') {
        if segment.is_empty() {
            return None;
        }
        node = node.as_object_mut()?.get_mut(segment)?;
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
