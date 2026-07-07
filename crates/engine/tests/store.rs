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
    // Empty schema serializes to an empty object.
    assert_eq!(cfg.all(), serde_json::json!({}));
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
    assert_eq!(record.id, "my-server");
    assert!(!record.ready);
    assert!(record.game_port.is_some());
    assert!(servers.create("My Server!", profile.clone(), None).is_err());

    let second = servers.create("Other", profile.clone(), None).unwrap();
    assert_ne!(second.game_port, record.game_port);
    assert!(servers
        .create("Third", profile.clone(), record.game_port)
        .is_err());
    assert!(servers.remove("other").unwrap());

    let ready = servers.mark_ready(&record.id).unwrap();
    assert!(ready.ready);
    assert_eq!(servers.get("my-server").unwrap().name, "My Server!");
    assert_eq!(servers.get("My Server!").unwrap().id, "my-server");
    assert_eq!(servers.list().len(), 1);
    assert_eq!(servers.list()[0].profile.game_version, profile.game_version);

    assert!(servers.remove("my-server").unwrap());
    assert!(!servers.remove("my-server").unwrap());
    assert!(servers.list().is_empty());
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
    assert_eq!(record.id, "modded");
    assert!(instances
        .create("modded", InstanceProfile::default())
        .is_err());

    assert_eq!(
        instances.get("Modded").unwrap().profile.loader_version,
        Some("0.16.5".into())
    );
    assert!(instances.instance_dir(&record.id).is_dir());

    assert!(instances.remove("modded").unwrap());
    assert!(instances.list().is_empty());
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
        instances.get("modded").unwrap().profile.game_version,
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
    let properties = servers.server_dir(id).join("server.properties");
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
