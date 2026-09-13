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

/// A shared selection would name packs the receiving instance has not installed.
pub const ALWAYS_LOCAL_KEYS: &[&str] = &["resourcePacks", "incompatibleResourcePacks"];
