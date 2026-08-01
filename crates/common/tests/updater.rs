//! The update feed is named once, in `common::app`. The desktop shell used to
//! carry a second copy in `tauri.conf.json` for `tauri-plugin-updater`, kept in
//! agreement by this test; the shell now asks the daemon like every other
//! front-end, so what is guarded is that the duplicate has not come back.

use std::path::Path;

fn tauri_conf() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../desktop/tauri.conf.json")
        .canonicalize()
        .expect("crates/desktop/tauri.conf.json is missing");
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

#[test]
fn the_shell_does_not_configure_its_own_updater() {
    assert!(
        tauri_conf()["plugins"]["updater"].is_null(),
        "tauri.conf.json declares plugins.updater again — the endpoint and the \
         signing key would be written in two places, which is what the \
         daemon-owned update path exists to avoid"
    );
}

#[test]
fn the_rotation_spare_is_a_different_key() {
    let next = common::app::UPDATE_PUBKEY_NEXT;
    assert!(
        next.is_empty() || next != common::app::UPDATE_PUBKEY,
        "UPDATE_PUBKEY_NEXT repeats the primary key — a rotation would have \
         nothing to rotate to"
    );
}

#[test]
fn the_primary_key_is_the_one_releases_are_signed_with() {
    assert!(
        common::app::update_pubkeys().next() == Some(common::app::UPDATE_PUBKEY),
        "the primary key must stay first: it is the one releases are signed with"
    );
}

/// The dev override is only ever reached with an overridden endpoint beside it,
/// so a stray `HESTIA_UPDATE_PUBKEY` in someone's shell cannot quietly change
/// what their build trusts. This test sets no endpoint, so the answer is the
/// compiled-in set whenever the cache happens to be filled.
#[test]
fn a_pubkey_override_alone_changes_nothing() {
    std::env::set_var("HESTIA_UPDATE_PUBKEY", "a-key-nobody-asked-for");
    let keys: Vec<_> = common::app::update_pubkeys().collect();
    std::env::remove_var("HESTIA_UPDATE_PUBKEY");
    assert_eq!(
        keys,
        vec![common::app::UPDATE_PUBKEY, common::app::UPDATE_PUBKEY_NEXT]
    );
}
