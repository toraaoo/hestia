//! Turns a resolved profile plus materialised paths into the JVM invocation:
//! classpath assembly and Mojang `${placeholder}` substitution. Pure functions —
//! spawning is the daemon supervisor's job.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use proto::minecraft::{InstanceProfile, ServerProfile};

// Mojang's rule vocabulary spells the classpath separator per-OS; keep ours in
// lockstep with the JVM's.
const CLASSPATH_SEPARATOR: &str = if cfg!(windows) { ";" } else { ":" };

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
pub fn server_plan(profile: &ServerProfile, java: &Path, dir: &Path) -> LaunchPlan {
    let mut args = Vec::new();
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
        let plan = server_plan(&profile, Path::new("/java/bin/java"), Path::new("/srv"));
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
        let plan = server_plan(&profile, Path::new("java"), Path::new("/srv"));
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
        let plan = instance_plan(&profile, Path::new("java"), &paths, &account());
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
        let plan = instance_plan(&profile, Path::new("java"), &paths, &account());
        assert_eq!(plan.args[0], "-cp");
        assert!(plan.args[1].contains("client.jar"));
    }
}
