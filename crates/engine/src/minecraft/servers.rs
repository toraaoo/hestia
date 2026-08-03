//! The instance's multiplayer list — `servers.dat`, the file the in-game
//! "Add Server" screen writes.
//!
//! Unlike `level.dat` this one is **uncompressed** NBT, and unlike every other
//! file Hestia keeps, it is not ours: the running game holds the list in memory
//! and writes the whole file back when it exits, so a write made underneath a
//! session is lost. Callers are expected to say so rather than to refuse — the
//! warning is raised where the sessions are known (the engine flow).
//!
//! Reading is best-effort in the same spirit as a world: a missing file is an
//! empty list (a fresh instance has never opened multiplayer), and a file that
//! cannot be parsed is reported as an error only to a caller that is writing,
//! since overwriting a list we failed to understand would discard it.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use proto::instance::ServerEntry;
use serde::{Deserialize, Serialize};

pub const FILE: &str = "servers.dat";

/// The file's root compound. `servers` is the only tag the game writes.
#[derive(Serialize, Deserialize, Default)]
struct ServerList {
    servers: Vec<Entry>,
}

/// One list row in the file's own spelling. Every field is optional in
/// practice: entries written by older versions and by third-party editors omit
/// what they do not set, and an omission must not hide the row.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Entry {
    name: String,
    ip: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    icon: String,
    #[serde(rename = "acceptTextures", skip_serializing_if = "Option::is_none")]
    accept_textures: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hidden: Option<i8>,
}

pub fn path(game_dir: &Path) -> PathBuf {
    game_dir.join(FILE)
}

/// The instance's multiplayer list in the file's own order. A missing file is
/// an empty list; an unreadable one logs and reads as empty, so a corrupt
/// `servers.dat` cannot make the instance itself unlistable.
pub fn read(game_dir: &Path) -> Vec<ServerEntry> {
    match parse(&path(game_dir)) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::debug!(instance_dir = %game_dir.display(), error = %e, "cannot read servers.dat");
            Vec::new()
        }
    }
}

/// The same read, but a parse failure is an error rather than an empty list —
/// the form a write must use, since rewriting the file from an empty list would
/// throw away a list we merely failed to decode.
pub fn read_strict(game_dir: &Path) -> Result<Vec<ServerEntry>> {
    parse(&path(game_dir))
}

fn parse(file: &Path) -> Result<Vec<ServerEntry>> {
    let bytes = match std::fs::read(file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("cannot read {}", file.display())),
    };
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let list: ServerList = fastnbt::from_bytes(&bytes)
        .with_context(|| format!("{} is not a readable server list", file.display()))?;
    Ok(list.servers.into_iter().map(into_proto).collect())
}

/// Write the list back whole, staged through a temp file so a failure leaves
/// the player's existing list intact.
pub fn write(game_dir: &Path, servers: &[ServerEntry]) -> Result<()> {
    let file = path(game_dir);
    let list = ServerList {
        servers: servers.iter().map(from_proto).collect(),
    };
    let bytes = fastnbt::to_bytes(&list).context("cannot encode the server list")?;
    let staging = file.with_extension("dat.part");
    std::fs::write(&staging, &bytes)
        .with_context(|| format!("cannot write {}", staging.display()))?;
    std::fs::rename(&staging, &file)
        .with_context(|| format!("cannot commit {}", file.display()))?;
    Ok(())
}

/// Find an entry by name or by address, case-insensitively — the two things a
/// person has in front of them when naming one.
pub fn find(servers: &[ServerEntry], reference: &str) -> Option<usize> {
    let reference = reference.trim();
    servers
        .iter()
        .position(|s| s.name.eq_ignore_ascii_case(reference))
        .or_else(|| {
            servers
                .iter()
                .position(|s| s.address.eq_ignore_ascii_case(reference))
        })
}

/// Move the entry at `index` to `position`, counted over the visible entries
/// only — the rows a person was shown. The game's hidden scratch rows are not
/// part of the list anyone arranges, so they keep the slots they are in.
/// `false` when the position is past the end, which leaves the list untouched.
pub fn reposition(servers: &mut Vec<ServerEntry>, index: usize, position: usize) -> bool {
    let entry = servers.remove(index);
    let slots: Vec<usize> = servers
        .iter()
        .enumerate()
        .filter(|(_, entry)| !entry.hidden)
        .map(|(index, _)| index)
        .collect();
    if position > slots.len() {
        servers.insert(index, entry);
        return false;
    }
    servers.insert(slots.get(position).copied().unwrap_or(servers.len()), entry);
    true
}

fn into_proto(entry: Entry) -> ServerEntry {
    ServerEntry {
        name: entry.name,
        address: entry.ip,
        icon: entry.icon,
        accept_textures: entry.accept_textures.unwrap_or(0) != 0,
        hidden: entry.hidden.unwrap_or(0) != 0,
    }
}

fn from_proto(entry: &ServerEntry) -> Entry {
    Entry {
        name: entry.name.clone(),
        ip: entry.address.clone(),
        icon: entry.icon.clone(),
        accept_textures: Some(i8::from(entry.accept_textures)),
        hidden: entry.hidden.then_some(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, address: &str) -> ServerEntry {
        ServerEntry {
            name: name.into(),
            address: address.into(),
            ..ServerEntry::default()
        }
    }

    #[test]
    fn a_list_round_trips_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let servers = vec![
            ServerEntry {
                accept_textures: true,
                icon: "aWNvbg==".into(),
                ..entry("Hermitcraft", "smp.example.net:25565")
            },
            entry("LAN box", "192.168.1.10"),
        ];
        write(dir.path(), &servers).unwrap();
        assert_eq!(read(dir.path()), servers);
    }

    #[test]
    fn a_missing_file_is_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(dir.path()).is_empty());
        assert!(read_strict(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn a_corrupt_file_reads_empty_but_refuses_a_write_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(path(dir.path()), b"not nbt at all").unwrap();
        assert!(read(dir.path()).is_empty());
        assert!(read_strict(dir.path()).is_err());
    }

    #[test]
    fn an_entry_moves_to_a_visible_position() {
        let mut servers = vec![
            entry("A", "a.net"),
            entry("B", "b.net"),
            entry("C", "c.net"),
        ];
        assert!(reposition(&mut servers, 2, 0));
        assert_eq!(names(&servers), ["C", "A", "B"]);
        assert!(reposition(&mut servers, 0, 2));
        assert_eq!(names(&servers), ["A", "B", "C"]);
    }

    #[test]
    fn a_hidden_row_is_not_counted_as_a_position() {
        let hidden = ServerEntry {
            hidden: true,
            ..entry("scratch", "direct.example.net")
        };
        let mut servers = vec![entry("A", "a.net"), hidden, entry("B", "b.net")];
        assert!(reposition(&mut servers, 2, 0));
        assert_eq!(names(&servers), ["B", "A", "scratch"]);
    }

    #[test]
    fn a_position_past_the_end_leaves_the_list_alone() {
        let mut servers = vec![entry("A", "a.net"), entry("B", "b.net")];
        assert!(!reposition(&mut servers, 0, 2));
        assert_eq!(names(&servers), ["A", "B"]);
    }

    fn names(servers: &[ServerEntry]) -> Vec<&str> {
        servers.iter().map(|s| s.name.as_str()).collect()
    }

    #[test]
    fn an_entry_is_found_by_name_or_address_ignoring_case() {
        let servers = vec![entry("Hermitcraft", "smp.example.net")];
        assert_eq!(find(&servers, "hermitcraft"), Some(0));
        assert_eq!(find(&servers, "SMP.example.net"), Some(0));
        assert_eq!(find(&servers, "nothing"), None);
    }
}
