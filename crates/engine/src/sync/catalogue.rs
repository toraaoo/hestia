//! The syncable units, the file each one is, and the version each needs.

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

/// 1.13 respelled every keybind, and an old client drops what it cannot parse;
/// hotbars and the command history simply do not exist before their version.
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

/// 1.20.5 replaced item tags with components; neither era reads the other.
const COMPONENT_ITEMS_SINCE: (u64, u64, u64) = (1, 20, 5);

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

/// Keys that describe the file, the machine or a moment, so a copy is wrong.
pub const NEVER_SHARED_KEYS: &[&str] = &[
    // A copy makes the client migrate settings it never needed to touch.
    "version",
    // Name packs the receiver does not have.
    "resourcePacks",
    "incompatibleResourcePacks",
    "lastServer",
    // Hardware, not preference.
    "soundDevice",
    "fullscreenResolution",
    "overrideWidth",
    "overrideHeight",
    // First-run prompts the receiver never saw.
    "startedCleanly",
    "tutorialStep",
    "joinedFirstServer",
    "onboardAccessibility",
    "showInventoryAchievementHint",
    "skipRealms32bitWarning",
    "skipFriendsListPromo",
    "hideBundleTutorial",
    // The launcher owns it.
    "skin",
    // A pre-1.8 world default.
    "difficulty",
];
