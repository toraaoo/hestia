//! `command_history.txt`. It is an append-only log, so the sides are unioned:
//! picking one would drop commands the other typed.

use std::path::Path;

use anyhow::Result;

use super::reconcile;

const LIMIT: usize = 50;

pub fn merge(baseline: &Path, store: &Path, data: &Path) -> Result<()> {
    let stored = read(store);
    let local = read(data);
    if stored.is_empty() && local.is_empty() {
        return Ok(());
    }

    let mut merged = stored;
    for line in local {
        merged.retain(|kept| kept != &line);
        merged.push(line);
    }
    if merged.len() > LIMIT {
        merged.drain(..merged.len() - LIMIT);
    }

    let text = merged.join("\n");
    let bytes = match text.is_empty() {
        true => Vec::new(),
        false => format!("{text}\n").into_bytes(),
    };
    reconcile::write_if_changed(data, &bytes)?;
    reconcile::write_if_changed(store, &bytes)?;
    reconcile::write_if_changed(baseline, &bytes)
}

fn read(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|text| {
            text.lines()
                .map(str::trim_end)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history(dir: &Path, name: &str, lines: &[&str]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, format!("{}\n", lines.join("\n"))).unwrap();
        path
    }

    #[test]
    fn the_union_is_capped_at_what_the_game_keeps() {
        let dir = tempfile::tempdir().unwrap();
        let stored: Vec<String> = (0..40).map(|n| format!("/stored {n}")).collect();
        let local: Vec<String> = (0..30).map(|n| format!("/local {n}")).collect();
        let store = history(
            dir.path(),
            "store.txt",
            &stored.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let data = history(
            dir.path(),
            "data.txt",
            &local.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let baseline = dir.path().join("baseline.txt");

        merge(&baseline, &store, &data).unwrap();

        let merged = std::fs::read_to_string(&data).unwrap();
        let lines: Vec<&str> = merged.lines().collect();
        assert_eq!(lines.len(), LIMIT);
        assert_eq!(lines.last(), Some(&"/local 29"));
        assert!(!merged.contains("/stored 0\n"));
    }

    #[test]
    fn a_command_typed_again_keeps_only_its_newest_place() {
        let dir = tempfile::tempdir().unwrap();
        let store = history(dir.path(), "store.txt", &["/one", "/two"]);
        let data = history(dir.path(), "data.txt", &["/one"]);
        let baseline = dir.path().join("baseline.txt");

        merge(&baseline, &store, &data).unwrap();

        assert_eq!(
            std::fs::read_to_string(&data).unwrap(),
            "/two\n/one\n",
            "the repeat moves to the end rather than duplicating"
        );
    }
}
