//! Behaviour-parity tests for the pure-filesystem engine stores (config, cache).

use engine::{Cache, Config};
use proto::download::{Checksum, HashAlgorithm};

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
