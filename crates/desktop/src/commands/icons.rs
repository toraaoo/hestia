//! Custom entry icons, copied into the hestia data home.
//!
//! A picked image is copied to `<data_home>/icons/<entry-id>.<ext>` so it
//! survives the original file moving; the disk is the registry (no index).
//! The webview loads them over the asset protocol, whose scope is widened to
//! the icons directory at each call — the data home can move at runtime.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::bridge::CallError;

const MAX_BYTES: u64 = 10 * 1024 * 1024;
const EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

/// One entry's stored icon; `mtime` doubles as the cache-busting version.
#[derive(Serialize, Clone)]
pub struct IconEntry {
    pub path: String,
    pub mtime: u64,
}

fn icons_dir() -> PathBuf {
    common::paths::data_home(None).join("icons")
}

// Entry ids are slug + hex tag; anything else could escape the icons dir.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn allow_assets(app: &AppHandle, dir: &Path) {
    let _ = app.asset_protocol_scope().allow_directory(dir, false);
}

fn entry_for(path: &Path) -> Option<IconEntry> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(IconEntry {
        path: path.to_string_lossy().into_owned(),
        mtime,
    })
}

fn stored_icons(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            let id = path.file_stem()?.to_str()?.to_string();
            path.is_file().then_some((id, path))
        })
        .collect()
}

fn remove_stored(dir: &Path, id: &str) {
    for (stored_id, path) in stored_icons(dir) {
        if stored_id == id {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[tauri::command]
pub fn icons_list(app: AppHandle) -> Result<BTreeMap<String, IconEntry>, CallError> {
    let dir = icons_dir();
    allow_assets(&app, &dir);
    Ok(stored_icons(&dir)
        .into_iter()
        .filter_map(|(id, path)| entry_for(&path).map(|entry| (id, entry)))
        .collect())
}

#[tauri::command]
pub fn icon_set(
    app: AppHandle,
    entry_id: String,
    source_path: String,
) -> Result<IconEntry, CallError> {
    if !valid_id(&entry_id) {
        return Err(CallError::other("invalid entry id"));
    }
    let source = PathBuf::from(&source_path);
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|e| EXTENSIONS.contains(&e.as_str()))
        .ok_or_else(|| CallError::other("unsupported image type"))?;
    let size = std::fs::metadata(&source)
        .map_err(|e| CallError::other(e.to_string()))?
        .len();
    if size > MAX_BYTES {
        return Err(CallError::other("image is larger than 10 MB"));
    }

    let dir = icons_dir();
    std::fs::create_dir_all(&dir).map_err(|e| CallError::other(e.to_string()))?;
    remove_stored(&dir, &entry_id);
    let target = dir.join(format!("{entry_id}.{ext}"));
    std::fs::copy(&source, &target).map_err(|e| CallError::other(e.to_string()))?;
    allow_assets(&app, &dir);
    entry_for(&target).ok_or_else(|| CallError::other("cannot read the stored icon"))
}

#[tauri::command]
pub fn icon_remove(entry_id: String) -> Result<(), CallError> {
    if !valid_id(&entry_id) {
        return Err(CallError::other("invalid entry id"));
    }
    remove_stored(&icons_dir(), &entry_id);
    Ok(())
}
