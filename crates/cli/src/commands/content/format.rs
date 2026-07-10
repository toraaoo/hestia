//! Label and number formatting shared by the browse and management views.

use client::proto::content::{ContentKind, InstalledContent, ReleaseChannel, SideSupport};

pub(super) fn source_label(item: &InstalledContent) -> String {
    if item.project_id.is_empty() {
        item.source.clone()
    } else {
        format!("{} ({})", item.source, item.version_number)
    }
}

pub(super) fn side_label(side: SideSupport) -> String {
    match side {
        SideSupport::Required => "required",
        SideSupport::Optional => "optional",
        SideSupport::Unsupported => "unsupported",
        SideSupport::Unknown => "unknown",
    }
    .to_string()
}

pub(super) fn channel_label(channel: ReleaseChannel) -> &'static str {
    match channel {
        ReleaseChannel::Release => "release",
        ReleaseChannel::Beta => "beta",
        ReleaseChannel::Alpha => "alpha",
    }
}

pub(super) fn kind_plural(kind: ContentKind) -> &'static str {
    match kind {
        ContentKind::Mod => "mods",
        ContentKind::Modpack => "modpacks",
        ContentKind::ResourcePack => "resourcepacks",
        ContentKind::Shader => "shaders",
        ContentKind::DataPack => "datapacks",
    }
}

/// A large count in compact units (180204729 → "180.2M").
pub(super) fn compact(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

pub(super) fn truncate(text: &str, max: usize) -> String {
    let flat = text.replace('\n', " ");
    if flat.chars().count() <= max {
        return flat;
    }
    let mut out: String = flat.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
