//! Drift guard: the desktop shell reads its update endpoint and minisign key
//! from `tauri.conf.json` (tauri-plugin-updater owns that file), while the
//! daemon and CLI read `common::app`. Both must name the same feed and trust
//! the same key, or a CLI self-update would verify against a key the desktop
//! never signs with.

use std::path::Path;

fn updater_config() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../desktop/tauri.conf.json")
        .canonicalize()
        .expect("crates/desktop/tauri.conf.json is missing");
    let conf: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    conf["plugins"]["updater"].clone()
}

#[test]
fn endpoint_matches_the_desktop_shell() {
    let updater = updater_config();
    assert_eq!(
        updater["endpoints"][0].as_str(),
        Some(common::app::UPDATE_ENDPOINT),
        "common::app::UPDATE_ENDPOINT and tauri.conf.json's updater endpoint disagree"
    );
}

#[test]
fn pubkey_matches_the_desktop_shell() {
    let updater = updater_config();
    assert_eq!(
        updater["pubkey"].as_str(),
        Some(common::app::UPDATE_PUBKEY),
        "common::app::UPDATE_PUBKEY and tauri.conf.json's updater pubkey disagree — \
         shipped clients would verify against different keys"
    );
}
