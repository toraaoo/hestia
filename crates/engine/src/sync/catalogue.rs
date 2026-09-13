//! The closed set of syncable units and the file each one is.

use proto::sync::SyncUnit;

pub const ALL: &[SyncUnit] = &[
    SyncUnit::Options,
    SyncUnit::Servers,
    SyncUnit::Commands,
    SyncUnit::Hotbars,
];

pub fn file(unit: SyncUnit) -> &'static str {
    match unit {
        SyncUnit::Options => "options.txt",
        SyncUnit::Servers => "servers.dat",
        SyncUnit::Commands => "command_history.txt",
        SyncUnit::Hotbars => "hotbar.nbt",
    }
}

/// 1.13 renamed every `options.txt` keybind from an LWJGL key code to a
/// `key.keyboard.*` name. An old client drops the binds it cannot parse and
/// writes its own spelling back, so the gate holds in both directions.
const SHARED_FORMATS_SINCE: (u64, u64, u64) = (1, 13, 0);

pub fn era_bound(unit: SyncUnit) -> bool {
    matches!(unit, SyncUnit::Options)
}

/// An id that is not a release triple is a snapshot, which is modern.
pub fn shares_era_bound(game_version: &str) -> bool {
    crate::version::parse(game_version).is_none_or(|v| v >= SHARED_FORMATS_SINCE)
}

pub fn captured(unit: SyncUnit) -> bool {
    matches!(unit, SyncUnit::Options)
}

/// `options.txt` keys that must never travel between instances, whatever the
/// catalogue says. Sharing one is not a preference the user can hold — it
/// either describes the file, the machine, or a moment, and copying it makes
/// the receiving instance wrong rather than merely different.
pub const NEVER_SHARED_KEYS: &[&str] = &[
    // The data version of the file itself. Copying another instance's tells
    // this client the file came from a different game version, and it migrates
    // keybinds and settings it never needed to touch.
    "version",
    // Name packs and worlds the receiving instance does not have.
    "resourcePacks",
    "incompatibleResourcePacks",
    "lastServer",
    // The machine's hardware, not the player's preference.
    "soundDevice",
    "fullscreenResolution",
    "overrideWidth",
    "overrideHeight",
    // First-run and one-off prompt state: copying it re-arms or suppresses a
    // prompt on an instance that never saw it.
    "startedCleanly",
    "tutorialStep",
    "joinedFirstServer",
    "onboardAccessibility",
    "showInventoryAchievementHint",
    "skipRealms32bitWarning",
    "skipFriendsListPromo",
    "hideBundleTutorial",
    // Account-scoped, and the launcher owns it.
    "skin",
    // A pre-1.8 world default that has nothing to do with the client.
    "difficulty",
];
