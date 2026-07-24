//! Behaviour-parity tests for the pure-filesystem engine stores (config, cache,
//! servers, instances).

use engine::{Cache, Config, Instances, Servers};
use proto::download::{Checksum, HashAlgorithm};
use proto::minecraft::{InstanceProfile, ServerProfile};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let base =
        std::env::temp_dir().join(format!("hestia-engine-test-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    base
}

#[test]
fn config_rejects_unknown_keys() {
    let dir = temp_dir("config");
    let cfg = Config::new(dir.join("config"));
    assert!(cfg.get("launcher.memory").is_err());
    assert!(cfg.set("launcher.memory", serde_json::json!(4096)).is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn config_jvm_defaults_validate_and_normalise() {
    let dir = temp_dir("config-defaults");
    let cfg = Config::new(dir.join("config"));

    cfg.set("defaults.memory", serde_json::json!("4g")).unwrap();
    assert_eq!(cfg.get("defaults.memory").unwrap(), serde_json::json!("4G"));
    assert!(cfg
        .set("defaults.memory", serde_json::json!("lots"))
        .is_err());

    cfg.set("defaults.jvm-args", serde_json::json!("-XX:+UseG1GC"))
        .unwrap();
    assert!(cfg
        .set("defaults.jvm-args", serde_json::json!("not-a-flag stray"))
        .is_err());

    // Clearing both leaves no default applied at launch.
    cfg.set("defaults.memory", serde_json::json!("")).unwrap();
    cfg.set("defaults.jvm-args", serde_json::json!("")).unwrap();
    let defaults = cfg.settings().java_defaults();
    assert!(defaults.memory.is_none());
    assert!(defaults.jvm_args.is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cache_stores_and_lists_by_checksum() {
    let dir = temp_dir("cache");
    let cache = Cache::new(dir.join("cache"));

    // sha256 of the empty input.
    let checksum = Checksum {
        algorithm: HashAlgorithm::Sha256,
        hex: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
    };
    let blob = dir.join("blob.bin");
    std::fs::write(&blob, b"").unwrap();

    assert!(cache.lookup(&checksum).is_none());
    cache.store(&blob, &checksum);
    assert!(cache.lookup(&checksum).is_some());

    let usage = cache.usage();
    assert_eq!(usage.entries, 1);

    let entries = cache.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].checksum.hex, checksum.hex);

    let freed = cache.clear();
    assert_eq!(freed.entries, 1);
    assert!(cache.lookup(&checksum).is_none());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn servers_store_round_trips_records() {
    let dir = temp_dir("servers");
    let servers = Servers::new(dir.join("servers"));
    assert!(servers.list().is_empty());

    let profile = ServerProfile {
        flavor: "vanilla".into(),
        game_version: "1.21.1".into(),
        java_major: 21,
        ..Default::default()
    };
    let record = servers.create("My Server!", profile.clone(), None).unwrap();
    assert!(
        record.id.len() == 32 && record.id.chars().all(|c| c.is_ascii_hexdigit()),
        "id is an opaque uuid, not the slug: {}",
        record.id
    );
    assert!(!record.ready);
    assert!(record.game_port.is_some());
    let entry_dir = servers.server_dir(&record);
    assert!(entry_dir.is_dir());
    assert!(
        entry_dir.ends_with("my-server"),
        "the directory is named for the slug, not the id: {}",
        entry_dir.display()
    );
    assert!(
        !servers.data_dir(&record).exists(),
        "data/ appears on demand, not at create"
    );
    assert!(servers.create("My Server!", profile.clone(), None).is_err());

    let second = servers.create("Other", profile.clone(), None).unwrap();
    assert_ne!(second.game_port, record.game_port);
    assert!(servers
        .create("Third", profile.clone(), record.game_port)
        .is_err());
    assert!(servers.remove(&second.id).unwrap());

    let ready = servers.mark_ready(&record.id).unwrap();
    assert!(ready.ready);
    assert_eq!(servers.get(&record.id).unwrap().name, "My Server!");
    assert_eq!(servers.get("My Server!").unwrap().id, record.id);
    // A reference resolves by any spelling that slugs to the name.
    assert_eq!(servers.get("my-server").unwrap().id, record.id);
    assert_eq!(servers.get("MY  server").unwrap().id, record.id);
    assert!(servers.get("nope").is_none());
    assert_eq!(servers.list().len(), 1);
    assert_eq!(servers.list()[0].profile.game_version, profile.game_version);

    assert!(servers.remove(&record.id).unwrap());
    assert!(!servers.remove(&record.id).unwrap());
    assert!(servers.list().is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn server_rename_keeps_the_id_moves_the_directory() {
    let dir = temp_dir("server-rename");
    let servers = Servers::new(dir.join("servers"));
    let profile = ServerProfile {
        flavor: "vanilla".into(),
        game_version: "1.21.1".into(),
        ..Default::default()
    };
    let created = servers.create("Old Name", profile.clone(), None).unwrap();
    let id = created.id.clone();
    let port = created.game_port;
    let old_dir = servers.server_dir(&created);
    servers.create("Keeper", profile, None).unwrap();

    // Renaming onto another entry's name is refused.
    assert!(servers.rename(&id, "Keeper").is_err());

    let renamed = servers.rename(&id, "New Name").unwrap();
    assert_eq!(renamed.id, id, "the id is stable across a rename");
    assert_eq!(renamed.name, "New Name");
    assert_eq!(renamed.game_port, port, "the claimed port is untouched");
    let new_dir = servers.server_dir(&renamed);
    assert!(new_dir.is_dir() && new_dir.ends_with("new-name"));
    assert!(!old_dir.exists(), "the birth-name directory is gone");
    assert_eq!(servers.get("New Name").unwrap().id, id);
    assert!(servers.get("Old Name").is_none());

    assert!(servers.rename("missing", "Whatever").is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn instances_store_round_trips_records() {
    let dir = temp_dir("instances");
    let instances = Instances::new(dir.join("instances"));
    assert!(instances.list().is_empty());

    let profile = InstanceProfile {
        flavor: "fabric".into(),
        game_version: "1.21.1".into(),
        loader_version: Some("0.16.5".into()),
        java_major: 21,
        ..Default::default()
    };
    let record = instances.create("Modded", profile).unwrap();
    assert!(
        record.id.len() == 32 && record.id.chars().all(|c| c.is_ascii_hexdigit()),
        "id is an opaque uuid, not the slug: {}",
        record.id
    );
    assert!(
        instances
            .create("modded", InstanceProfile::default())
            .is_err(),
        "a slug-colliding name is refused"
    );

    assert_eq!(
        instances.get("Modded").unwrap().profile.loader_version,
        Some("0.16.5".into())
    );
    let entry_dir = instances.instance_dir(&record);
    assert!(entry_dir.is_dir() && entry_dir.ends_with("modded"));
    assert_eq!(instances.data_dir(&record), entry_dir.join("data"));
    assert!(
        !instances.data_dir(&record).exists(),
        "data/ appears on demand, not at create"
    );

    assert!(instances.remove(&record.id).unwrap());
    assert!(instances.list().is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn instance_rename_keeps_the_id_moves_the_directory() {
    let dir = temp_dir("instance-rename");
    let instances = Instances::new(dir.join("instances"));
    let record = instances
        .create(
            "Old Pack",
            InstanceProfile {
                flavor: "fabric".into(),
                game_version: "1.21.1".into(),
                ..Default::default()
            },
        )
        .unwrap();
    let id = record.id.clone();
    let old_dir = instances.instance_dir(&record);
    instances.config_set(&id, "memory", "4G").unwrap();
    instances
        .create("Keeper", InstanceProfile::default())
        .unwrap();

    assert!(instances.rename(&id, "Keeper").is_err());

    let renamed = instances.rename(&id, "New Pack").unwrap();
    assert_eq!(renamed.id, id, "the id is stable across a rename");
    assert_eq!(renamed.name, "New Pack");
    let new_dir = instances.instance_dir(&renamed);
    assert!(new_dir.is_dir() && new_dir.ends_with("new-pack"));
    assert!(!old_dir.exists(), "the birth-name directory is gone");
    assert_eq!(instances.get("New Pack").unwrap().id, id);
    assert!(instances.get("Old Pack").is_none());
    assert_eq!(
        instances.config_get(&id, "memory").unwrap().as_deref(),
        Some("4G"),
        "JVM settings are untouched"
    );
    assert!(instances.rename("missing", "Whatever").is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn instance_update_swaps_profile_and_keeps_settings() {
    let dir = temp_dir("instance-update");
    let instances = Instances::new(dir.join("instances"));
    let record = instances
        .create(
            "Modded",
            InstanceProfile {
                flavor: "fabric".into(),
                game_version: "1.21.1".into(),
                loader_version: Some("0.16.5".into()),
                ..Default::default()
            },
        )
        .unwrap();
    instances.config_set(&record.id, "memory", "4G").unwrap();

    let updated = instances
        .update(
            &record.id,
            InstanceProfile {
                flavor: "fabric".into(),
                game_version: "1.21.4".into(),
                loader_version: Some("0.16.9".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(updated.name, "Modded");
    assert_eq!(updated.profile.game_version, "1.21.4");

    assert_eq!(
        instances.get(&record.id).unwrap().profile.game_version,
        "1.21.4"
    );
    assert_eq!(
        instances
            .config_get(&record.id, "memory")
            .unwrap()
            .as_deref(),
        Some("4G"),
        "JVM settings survive an update"
    );
    assert!(instances
        .update("missing", InstanceProfile::default())
        .is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn server_config_covers_jvm_and_properties() {
    let dir = temp_dir("server-config");
    let servers = Servers::new(dir.join("servers"));
    let record = servers
        .create("SMP", ServerProfile::default(), None)
        .unwrap();
    let id = &record.id;

    // JVM keys live on the record; unset reads back as None.
    assert_eq!(servers.config_get(id, "memory").unwrap(), None);
    servers.config_set(id, "memory", "4g").unwrap();
    assert_eq!(
        servers.config_get(id, "memory").unwrap().as_deref(),
        Some("4G")
    );

    // A hestia-managed property is rejected before any other check, file or
    // no file.
    let managed = servers.config_set(id, "server-port", "25570").unwrap_err();
    assert!(managed.to_string().contains("managed by hestia"));

    // No generated server.properties yet: there is no ground truth, so any
    // (unmanaged) key is accepted rather than everything rejected.
    servers.config_set(id, "no-schema-yet", "1").unwrap();
    assert_eq!(
        servers.config_get(id, "no-schema-yet").unwrap().as_deref(),
        Some("1")
    );

    // Seed the file as the generation run would: every key the server's
    // version knows, with its default.
    let properties = servers.data_dir(&record).join("server.properties");
    std::fs::create_dir_all(properties.parent().unwrap()).unwrap();
    std::fs::write(&properties, "motd=A Minecraft Server\nview-distance=10\n").unwrap();

    // A key in the generated schema is accepted and reads back.
    servers.config_set(id, "motd", "hi there").unwrap();
    assert_eq!(
        servers.config_get(id, "motd").unwrap().as_deref(),
        Some("hi there")
    );

    // A key the schema does not carry is rejected — a typo cannot drift the
    // file.
    let unknown = servers.config_set(id, "this-is-a-typo", "x").unwrap_err();
    assert!(unknown.to_string().contains("this server's version"));

    // Managed keys stay rejected even though the file now exists.
    assert!(servers.config_set(id, "rcon.port", "25580").is_err());

    // list surfaces both reserved JVM keys plus the properties entries.
    let entries = servers.config_list(id).unwrap();
    assert!(entries.iter().any(|(k, v)| k == "memory" && v == "4G"));
    assert!(entries.iter().any(|(k, _)| k == "jvm-args"));
    assert!(entries.iter().any(|(k, v)| k == "motd" && v == "hi there"));
    assert!(entries
        .iter()
        .any(|(k, v)| k == "view-distance" && v == "10"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn instance_config_rejects_unknown_keys() {
    let dir = temp_dir("instance-config");
    let instances = Instances::new(dir.join("instances"));
    let record = instances
        .create("Modded", InstanceProfile::default())
        .unwrap();
    let id = &record.id;

    instances
        .config_set(id, "jvm-args", "-XX:+UseG1GC -Xmn1G")
        .unwrap();
    assert_eq!(
        instances.config_get(id, "jvm-args").unwrap().as_deref(),
        Some("-XX:+UseG1GC -Xmn1G")
    );
    // Empty clears.
    instances.config_set(id, "jvm-args", "").unwrap();
    assert_eq!(instances.config_get(id, "jvm-args").unwrap(), None);

    // Non-JVM keys are rejected (no properties file for instances).
    assert!(instances.config_set(id, "motd", "hi").is_err());
    assert!(instances.config_get(id, "motd").is_err());

    std::fs::remove_dir_all(&dir).ok();
}
