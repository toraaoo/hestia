//! Screenshots are read where the game wrote them, so the webview has to be let
//! into each instance's own folder before an `asset:` URL resolves.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

const DIR: &str = "screenshots";

#[tauri::command]
pub fn screenshots_allow(app: AppHandle, dirs: Vec<String>) -> Result<(), String> {
    let home = common::paths::data_home(None);
    for dir in dirs {
        let path = PathBuf::from(dir);
        if !is_screenshots_dir(&home, &path) {
            return Err(format!("{} is not a screenshots folder", path.display()));
        }
        app.asset_protocol_scope()
            .allow_directory(&path, false)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn is_screenshots_dir(home: &Path, path: &Path) -> bool {
    let Ok(path) = path.canonicalize() else {
        return false;
    };
    let Ok(home) = home.canonicalize() else {
        return false;
    };
    path.starts_with(&home) && path.file_name().is_some_and(|name| name == DIR)
}
