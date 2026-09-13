//! The closed set of syncable units, the file each one is, and what a unit
//! needs from the game version it is asked to share with.

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

/// The version each unit's file first exists in, or first exists in a spelling
/// the other instances can read. Below it the file is either absent (the game
/// never writes one) or written in a form the modern one corrupts, and sharing
/// it degrades whichever side wrote last.
///
/// - `options.txt`: 1.13 renamed every keybind from an LWJGL key code to a
///   `key.keyboard.*` name. An old client drops the binds it cannot parse and
///   writes its own spelling back, so the gate holds in both directions.
/// - `hotbar.nbt`: creative hotbar saving arrived in 1.12.
/// - `command_history.txt`: the game only started writing one in 1.20.2.
fn requires(unit: SyncUnit) -> Option<(u64, u64, u64)> {
    match unit {
        SyncUnit::Options => Some((1, 13, 0)),
        SyncUnit::Hotbars => Some((1, 12, 0)),
        SyncUnit::Commands => Some((1, 20, 2)),
        SyncUnit::Servers => None,
    }
}

/// What a front-end tells the user a skipped unit is waiting for.
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
