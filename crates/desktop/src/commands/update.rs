/// This build's own release notes, compiled in from `CHANGELOG.md`. Empty when
/// the changelog has no section for it.
///
/// Deliberately local: the shell shows these on the first run *after* an
/// update, which is precisely when the network may be unreliable.
#[tauri::command]
pub fn changelog() -> String {
    common::changelog::for_version(common::app::VERSION)
        .unwrap_or_default()
        .to_string()
}
