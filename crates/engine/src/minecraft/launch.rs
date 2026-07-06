//! Turns a resolved profile plus materialised paths into the JVM invocation:
//! classpath assembly and Mojang `${placeholder}` substitution. Pure functions —
//! spawning is the daemon supervisor's job.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use proto::minecraft::{InstanceProfile, ServerProfile};
use serde::{Deserialize, Serialize};

// Mojang's rule vocabulary spells the classpath separator per-OS; keep ours in
// lockstep with the JVM's.
const CLASSPATH_SEPARATOR: &str = if cfg!(windows) { ";" } else { ":" };

/// The reserved per-entry JVM setting keys, shared by the server and instance
/// `config` surfaces.
pub const MEMORY_KEY: &str = "memory";
pub const JVM_ARGS_KEY: &str = "jvm-args";

/// Per-entry JVM tuning stored on a server/instance record: a single memory
/// value driving both `-Xms`/`-Xmx`, plus extra flags injected at launch.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct JavaSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub jvm_args: Vec<String>,
}

impl JavaSettings {
    /// The current value of a JVM key, or `None` (outer) when `key` is not one
    /// of the reserved JVM keys — the signal to fall through to properties. The
    /// inner `None` means the key is a JVM key but unset.
    pub fn get(&self, key: &str) -> Option<Option<String>> {
        match key {
            MEMORY_KEY => Some(self.memory.clone()),
            JVM_ARGS_KEY => Some(if self.jvm_args.is_empty() {
                None
            } else {
                Some(self.jvm_args.join(" "))
            }),
            _ => None,
        }
    }

    /// Apply a JVM key. `Ok(false)` means `key` is not a JVM key (fall through
    /// to properties); an empty value clears the setting; an invalid value is
    /// `Err`.
    pub fn set(&mut self, key: &str, value: &str) -> Result<bool> {
        match key {
            MEMORY_KEY => {
                self.memory = if value.trim().is_empty() {
                    None
                } else {
                    Some(normalize_memory(value)?)
                };
                Ok(true)
            }
            JVM_ARGS_KEY => {
                self.jvm_args = parse_jvm_args(value)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Both reserved keys with their current values (empty when unset), so a
    /// `config list` always shows what is settable.
    pub fn entries(&self) -> Vec<(String, String)> {
        vec![
            (
                MEMORY_KEY.to_string(),
                self.memory.clone().unwrap_or_default(),
            ),
            (JVM_ARGS_KEY.to_string(), self.jvm_args.join(" ")),
        ]
    }

    /// The JVM flags these settings inject: the memory pair first, then the
    /// extra args.
    pub fn flags(&self) -> Vec<String> {
        let mut flags = Vec::new();
        if let Some(memory) = &self.memory {
            flags.push(format!("-Xms{memory}"));
            flags.push(format!("-Xmx{memory}"));
        }
        flags.extend(self.jvm_args.iter().cloned());
        flags
    }
}

/// Validate and normalise a memory value: digits followed by one unit char
/// (k/m/g, case-insensitive), the number nonzero, the unit upper-cased.
fn normalize_memory(value: &str) -> Result<String> {
    let trimmed = value.trim();
    let unit = trimmed
        .chars()
        .last()
        .filter(|c| "kmgKMG".contains(*c))
        .context("memory must look like 4G or 2048M")?;
    let digits = &trimmed[..trimmed.len() - unit.len_utf8()];
    let valid = !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && digits.bytes().any(|b| b != b'0');
    if !valid {
        bail!("memory must look like 4G or 2048M");
    }
    Ok(format!("{digits}{}", unit.to_ascii_uppercase()))
}

/// Split JVM args on whitespace; every token must start with `-`.
fn parse_jvm_args(value: &str) -> Result<Vec<String>> {
    let mut args = Vec::new();
    for token in value.split_whitespace() {
        if !token.starts_with('-') {
            bail!("jvm arguments must start with '-' (got '{token}')");
        }
        args.push(token.to_string());
    }
    Ok(args)
}

/// A ready-to-spawn process: program + args + working directory.
#[derive(Debug, Clone)]
pub struct LaunchPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

/// The signed-in identity substituted into the client's auth placeholders.
pub struct LaunchAccount {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
}

/// Where the materialised instance pieces live.
pub struct InstancePaths<'a> {
    pub game_dir: &'a Path,
    pub natives_dir: &'a Path,
    pub client_jar: &'a Path,
    pub libraries_root: &'a Path,
    pub assets_root: &'a Path,
}

/// The server invocation, run from the server directory. A profile without a
/// main class is a self-contained jar (`-jar <primary> nogui`); one with a main
/// class runs off a classpath of its libraries plus the primary.
pub fn server_plan(
    profile: &ServerProfile,
    java: &Path,
    dir: &Path,
    settings: &JavaSettings,
) -> LaunchPlan {
    let mut args = settings.flags();
    if profile.main_class.is_empty() {
        args.push("-jar".to_string());
        args.push(profile.primary.filename.clone());
    } else {
        let mut entries: Vec<String> = Vec::new();
        for library in &profile.libraries {
            push_unique(
                &mut entries,
                join_str(Path::new("libraries"), &library.path),
            );
        }
        entries.push(profile.primary.filename.clone());
        args.push("-cp".to_string());
        args.push(entries.join(CLASSPATH_SEPARATOR));
        args.push(profile.main_class.clone());
    }
    args.push("nogui".to_string());
    LaunchPlan {
        program: java.to_path_buf(),
        args,
        cwd: dir.to_path_buf(),
    }
}

/// The client invocation: substituted JVM args, the main class, then
/// substituted game args, run from the instance's game directory.
pub fn instance_plan(
    profile: &InstanceProfile,
    java: &Path,
    paths: &InstancePaths<'_>,
    account: &LaunchAccount,
    settings: &JavaSettings,
) -> LaunchPlan {
    let classpath = build_classpath(profile, paths);
    let vars = build_vars(profile, paths, account, classpath);

    let mut args = Vec::new();
    if profile.jvm_args.is_empty() {
        // Pre-`arguments` manifests carry no JVM section; supply the classpath.
        args.push("-cp".to_string());
        args.push(vars["classpath"].clone());
    } else {
        args.extend(profile.jvm_args.iter().map(|a| substitute(a, &vars)));
    }
    // User flags come last in the JVM section so they win over the manifest.
    args.extend(settings.flags());
    args.push(profile.main_class.clone());
    args.extend(profile.game_args.iter().map(|a| substitute(a, &vars)));

    LaunchPlan {
        program: java.to_path_buf(),
        args,
        cwd: paths.game_dir.to_path_buf(),
    }
}

fn build_classpath(profile: &InstanceProfile, paths: &InstancePaths<'_>) -> String {
    let mut entries: Vec<String> = Vec::new();
    for library in &profile.libraries {
        push_unique(&mut entries, join_str(paths.libraries_root, &library.path));
    }
    entries.push(paths.client_jar.to_string_lossy().into_owned());
    entries.join(CLASSPATH_SEPARATOR)
}

fn build_vars(
    profile: &InstanceProfile,
    paths: &InstancePaths<'_>,
    account: &LaunchAccount,
    classpath: String,
) -> HashMap<&'static str, String> {
    let path_str = |p: &Path| p.to_string_lossy().into_owned();
    HashMap::from([
        ("classpath", classpath),
        ("natives_directory", path_str(paths.natives_dir)),
        ("game_directory", path_str(paths.game_dir)),
        ("assets_root", path_str(paths.assets_root)),
        ("assets_index_name", profile.asset_index.id.clone()),
        ("version_name", profile.game_version.clone()),
        ("version_type", "release".to_string()),
        ("launcher_name", common::app::NAME.to_string()),
        ("launcher_version", common::app::VERSION.to_string()),
        ("auth_player_name", account.name.clone()),
        ("auth_uuid", account.uuid.clone()),
        ("auth_access_token", account.access_token.clone()),
        ("auth_session", account.access_token.clone()),
        ("user_type", "msa".to_string()),
        ("user_properties", "{}".to_string()),
        ("clientid", String::new()),
        ("auth_xuid", String::new()),
    ])
}

/// Replace every `${key}` with its variable; an unknown key becomes empty (and
/// is logged) rather than leaking the placeholder to the JVM.
fn substitute(arg: &str, vars: &HashMap<&'static str, String>) -> String {
    let mut out = String::with_capacity(arg.len());
    let mut rest = arg;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                match vars.get(key) {
                    Some(value) => out.push_str(value),
                    None => tracing::debug!(key, "unknown launch placeholder"),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

fn join_str(root: &Path, relative: &str) -> String {
    root.join(relative).to_string_lossy().into_owned()
}

fn push_unique(entries: &mut Vec<String>, entry: String) {
    if !entries.contains(&entry) {
        entries.push(entry);
    }
}

#[cfg(test)]
mod tests {
    use proto::minecraft::{Artifact, AssetIndex, Library};

    use super::*;

    fn account() -> LaunchAccount {
        LaunchAccount {
            name: "Steve".into(),
            uuid: "uuid-1".into(),
            access_token: "token-1".into(),
        }
    }

    fn library(path: &str) -> Library {
        Library {
            name: path.to_string(),
            path: path.to_string(),
            artifact: Artifact::default(),
        }
    }

    #[test]
    fn substitute_replaces_known_and_drops_unknown() {
        let vars = HashMap::from([("version_name", "1.21.1".to_string())]);
        assert_eq!(
            substitute("--version=${version_name}", &vars),
            "--version=1.21.1"
        );
        assert_eq!(substitute("${unknown_key}", &vars), "");
        assert_eq!(substitute("${broken", &vars), "${broken");
        assert_eq!(substitute("plain", &vars), "plain");
    }

    #[test]
    fn server_plan_without_main_class_is_a_jar_invocation() {
        let profile = ServerProfile {
            primary: Artifact {
                filename: "server.jar".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let plan = server_plan(
            &profile,
            Path::new("/java/bin/java"),
            Path::new("/srv"),
            &JavaSettings::default(),
        );
        assert_eq!(plan.args, ["-jar", "server.jar", "nogui"]);
        assert_eq!(plan.cwd, Path::new("/srv"));
    }

    #[test]
    fn server_plan_with_main_class_builds_a_classpath() {
        let profile = ServerProfile {
            primary: Artifact {
                filename: "launcher.jar".into(),
                ..Default::default()
            },
            libraries: vec![library("a/b.jar"), library("a/b.jar"), library("c/d.jar")],
            main_class: "net.example.Main".into(),
            ..Default::default()
        };
        let plan = server_plan(
            &profile,
            Path::new("java"),
            Path::new("/srv"),
            &JavaSettings::default(),
        );
        assert_eq!(plan.args[0], "-cp");
        let classpath = &plan.args[1];
        assert_eq!(classpath.matches("b.jar").count(), 1, "duplicates dropped");
        assert!(classpath.contains("launcher.jar"));
        assert_eq!(plan.args[2], "net.example.Main");
        assert_eq!(plan.args.last().map(String::as_str), Some("nogui"));
    }

    #[test]
    fn instance_plan_substitutes_and_orders_sections() {
        let profile = InstanceProfile {
            game_version: "1.21.1".into(),
            client: Artifact::default(),
            libraries: vec![library("a/b.jar")],
            asset_index: AssetIndex {
                id: "17".into(),
                ..Default::default()
            },
            main_class: "net.minecraft.client.main.Main".into(),
            jvm_args: vec!["-cp".into(), "${classpath}".into()],
            game_args: vec!["--username".into(), "${auth_player_name}".into()],
            ..Default::default()
        };
        let paths = InstancePaths {
            game_dir: Path::new("/inst"),
            natives_dir: Path::new("/inst/natives"),
            client_jar: Path::new("/versions/1.21.1/client.jar"),
            libraries_root: Path::new("/libraries"),
            assets_root: Path::new("/assets"),
        };
        let plan = instance_plan(
            &profile,
            Path::new("java"),
            &paths,
            &account(),
            &JavaSettings::default(),
        );
        let main_at = plan
            .args
            .iter()
            .position(|a| a == "net.minecraft.client.main.Main")
            .expect("main class present");
        assert!(plan.args[..main_at]
            .iter()
            .any(|a| a.contains("client.jar")));
        assert_eq!(plan.args[main_at + 1..], ["--username", "Steve"]);
        assert_eq!(plan.cwd, Path::new("/inst"));
    }

    #[test]
    fn instance_plan_supplies_classpath_when_jvm_args_absent() {
        let profile = InstanceProfile {
            main_class: "net.minecraft.client.Minecraft".into(),
            ..Default::default()
        };
        let paths = InstancePaths {
            game_dir: Path::new("/inst"),
            natives_dir: Path::new("/inst/natives"),
            client_jar: Path::new("/versions/old/client.jar"),
            libraries_root: Path::new("/libraries"),
            assets_root: Path::new("/assets"),
        };
        let plan = instance_plan(
            &profile,
            Path::new("java"),
            &paths,
            &account(),
            &JavaSettings::default(),
        );
        assert_eq!(plan.args[0], "-cp");
        assert!(plan.args[1].contains("client.jar"));
    }

    fn settings(memory: Option<&str>, jvm_args: &[&str]) -> JavaSettings {
        JavaSettings {
            memory: memory.map(str::to_string),
            jvm_args: jvm_args.iter().map(|a| a.to_string()).collect(),
        }
    }

    #[test]
    fn server_plan_prepends_memory_and_extra_flags() {
        let profile = ServerProfile {
            primary: Artifact {
                filename: "server.jar".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let plan = server_plan(
            &profile,
            Path::new("java"),
            Path::new("/srv"),
            &settings(Some("4G"), &["-XX:+UseG1GC"]),
        );
        assert_eq!(
            plan.args,
            [
                "-Xms4G",
                "-Xmx4G",
                "-XX:+UseG1GC",
                "-jar",
                "server.jar",
                "nogui"
            ]
        );
    }

    #[test]
    fn instance_plan_appends_user_flags_after_manifest_jvm_args() {
        let profile = InstanceProfile {
            client: Artifact::default(),
            main_class: "net.minecraft.client.main.Main".into(),
            jvm_args: vec!["-cp".into(), "${classpath}".into()],
            ..Default::default()
        };
        let paths = InstancePaths {
            game_dir: Path::new("/inst"),
            natives_dir: Path::new("/inst/natives"),
            client_jar: Path::new("/versions/1.21.1/client.jar"),
            libraries_root: Path::new("/libraries"),
            assets_root: Path::new("/assets"),
        };
        let plan = instance_plan(
            &profile,
            Path::new("java"),
            &paths,
            &account(),
            &settings(Some("2048M"), &["-XX:+UseZGC"]),
        );
        let main_at = plan
            .args
            .iter()
            .position(|a| a == "net.minecraft.client.main.Main")
            .expect("main class present");
        // The injected flags are the three args immediately before the main
        // class, after the manifest's jvm section.
        assert_eq!(
            &plan.args[main_at - 3..main_at],
            ["-Xms2048M", "-Xmx2048M", "-XX:+UseZGC"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .as_slice()
        );
        let cp_at = plan.args.iter().position(|a| a == "-cp").unwrap();
        assert!(cp_at < main_at - 3, "manifest jvm args precede user flags");
    }

    #[test]
    fn memory_validation_normalizes_accepts_and_rejects() {
        let mut s = JavaSettings::default();
        assert!(s.set(MEMORY_KEY, "4g").unwrap());
        assert_eq!(s.memory.as_deref(), Some("4G"));
        assert_eq!(s.flags(), ["-Xms4G", "-Xmx4G"]);
        assert!(s.set(MEMORY_KEY, "2048M").unwrap());
        assert_eq!(s.memory.as_deref(), Some("2048M"));
        // Empty clears.
        assert!(s.set(MEMORY_KEY, "").unwrap());
        assert_eq!(s.memory, None);
        for bad in ["4", "G", "0G", "4GB", "4.5G", "-4G", "four"] {
            assert!(s.set(MEMORY_KEY, bad).is_err(), "'{bad}' must be rejected");
        }
    }

    #[test]
    fn jvm_args_split_validate_and_clear() {
        let mut s = JavaSettings::default();
        assert!(s.set(JVM_ARGS_KEY, "-XX:+UseG1GC  -Xmn1G").unwrap());
        assert_eq!(s.jvm_args, ["-XX:+UseG1GC", "-Xmn1G"]);
        assert_eq!(
            s.get(JVM_ARGS_KEY),
            Some(Some("-XX:+UseG1GC -Xmn1G".into()))
        );
        assert!(s.set(JVM_ARGS_KEY, "notaflag").is_err());
        assert!(s.set(JVM_ARGS_KEY, "").unwrap());
        assert!(s.jvm_args.is_empty());
        assert_eq!(s.get(JVM_ARGS_KEY), Some(None));
    }

    #[test]
    fn get_returns_outer_none_for_non_jvm_key() {
        assert_eq!(JavaSettings::default().get("server-port"), None);
    }
}
