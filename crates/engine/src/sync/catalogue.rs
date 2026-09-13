//! The closed set of syncable units, the file each one is, and the version
//! each needs.

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

/// The version each unit's file first appears in, in a spelling the others can
/// read:
///
/// - `options.txt`: 1.13 renamed every keybind from an LWJGL code to a
///   `key.keyboard.*` name, and an old client drops what it cannot parse.
/// - `hotbar.nbt`: creative hotbars arrived in 1.12.
/// - `command_history.txt`: first written in 1.20.2.
fn requires(unit: SyncUnit) -> Option<(u64, u64, u64)> {
    match unit {
        SyncUnit::Options => Some((1, 13, 0)),
        SyncUnit::Hotbars => Some((1, 12, 0)),
        SyncUnit::Commands => Some((1, 20, 2)),
        SyncUnit::Servers => None,
    }
}

pub fn requirement(unit: SyncUnit) -> String {
    match requires(unit) {
        Some((major, minor, 0)) => format!("{major}.{minor}"),
        Some((major, minor, patch)) => format!("{major}.{minor}.{patch}"),
        None => String::new(),
    }
}

/// An id that is not a release triple is a snapshot, which is modern.
pub fn supports(unit: SyncUnit, game_version: &str) -> bool {
    let Some(floor) = requires(unit) else {
        return true;
    };
    crate::version::parse(game_version).is_none_or(|version| version >= floor)
}

/// 1.20.5 replaced an item's tags with components: a hotbar from either side
/// of it is not a file the other can read, so each era keeps its own copy.
const COMPONENT_ITEMS_SINCE: (u64, u64, u64) = (1, 20, 5);

/// Empty for a unit that has only ever had one form.
pub fn era(unit: SyncUnit, game_version: &str) -> &'static str {
    if unit != SyncUnit::Hotbars {
        return "";
    }
    match crate::version::parse(game_version) {
        Some(version) if version < COMPONENT_ITEMS_SINCE => "legacy",
        _ => "components",
    }
}

pub fn captured(unit: SyncUnit) -> bool {
    matches!(unit, SyncUnit::Options)
}

/// Keys that must never travel, whatever the catalogue says: each describes the
/// file, the machine or a moment, so a copy makes the receiver wrong.
pub const NEVER_SHARED_KEYS: &[&str] = &[
    // The file's own data version: a copy makes the client migrate settings it
    // never needed to touch.
    "version",
    // Name packs the receiving instance does not have.
    "resourcePacks",
    "incompatibleResourcePacks",
    "lastServer",
    // The machine's hardware, not the player's preference.
    "soundDevice",
    "fullscreenResolution",
    "overrideWidth",
    "overrideHeight",
    // First-run prompts: a copy re-arms one on an instance that never saw it.
    "startedCleanly",
    "tutorialStep",
    "joinedFirstServer",
    "onboardAccessibility",
    "showInventoryAchievementHint",
    "skipRealms32bitWarning",
    "skipFriendsListPromo",
    "hideBundleTutorial",
    // Account-scoped; the launcher owns it.
    "skin",
    // A pre-1.8 world default, not a client setting.
    "difficulty",
];
