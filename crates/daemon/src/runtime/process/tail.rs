//! File tailing: live `process.output` events come from polling the process's
//! log file (its own latest.log or the supervisor's capture file), and
//! `process.logs` reads the file's tail on demand.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ipc::protocol::Event;
use proto::process::{LogStream, ProcessLogLine, ProcessOutputEvent};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::runtime::event_hub::EventHub;

const POLL: Duration = Duration::from_millis(250);
const CHUNK: usize = 64 * 1024;

pub struct Tailer {
    done: Arc<AtomicBool>,
    wake: Arc<Notify>,
    handle: JoinHandle<()>,
}

impl Tailer {
    /// Must complete before the exit event is published, or subscribers miss
    /// trailing lines.
    pub async fn finish(self) {
        self.done.store(true, Ordering::SeqCst);
        self.wake.notify_one();
        let _ = self.handle.await;
    }
}

pub fn spawn(path: PathBuf, from: u64, id: String, hub: Arc<EventHub>) -> Tailer {
    let done = Arc::new(AtomicBool::new(false));
    let wake = Arc::new(Notify::new());
    let handle = tokio::spawn(run(path, from, id, hub, done.clone(), wake.clone()));
    Tailer { done, wake, handle }
}

async fn run(
    path: PathBuf,
    mut offset: u64,
    id: String,
    hub: Arc<EventHub>,
    done: Arc<AtomicBool>,
    wake: Arc<Notify>,
) {
    let mut pending = Vec::new();
    loop {
        let finishing = done.load(Ordering::SeqCst);
        drain(&path, &mut offset, &mut pending, |line| {
            hub.publish(&output_event(&id, line));
        });
        if finishing {
            break;
        }
        tokio::select! {
            _ = tokio::time::sleep(POLL) => {}
            _ = wake.notified() => {}
        }
    }
}

fn output_event(id: &str, line: String) -> Event {
    let event = ProcessOutputEvent {
        id: id.to_string(),
        line: ProcessLogLine {
            stream: LogStream::Stdout,
            line,
        },
    };
    Event {
        topic: <ProcessOutputEvent as proto::Topic>::TOPIC.to_string(),
        payload: serde_json::to_value(&event).unwrap_or_default(),
    }
}

fn drain(path: &Path, offset: &mut u64, pending: &mut Vec<u8>, mut emit: impl FnMut(String)) {
    let Ok(mut file) = File::open(path) else {
        return;
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    if len < *offset {
        // Rotated or truncated (log4j archives latest.log): start over.
        *offset = 0;
        pending.clear();
    }
    if len == *offset || file.seek(SeekFrom::Start(*offset)).is_err() {
        return;
    }
    let mut remaining = (len - *offset) as usize;
    let mut chunk = vec![0u8; CHUNK.min(remaining)];
    while remaining > 0 {
        let want = CHUNK.min(remaining);
        let Ok(read) = file.read(&mut chunk[..want]) else {
            break;
        };
        if read == 0 {
            break;
        }
        *offset += read as u64;
        remaining -= read;
        pending.extend_from_slice(&chunk[..read]);
        while let Some(pos) = pending.iter().position(|&b| b == b'\n') {
            let mut line: Vec<u8> = pending.drain(..=pos).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            emit(String::from_utf8_lossy(&line).into_owned());
        }
    }
}

/// The last `limit` complete lines of `path`; empty when the file is missing.
pub fn read_last_lines(path: &Path, limit: usize) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    let Ok(mut file) = File::open(path) else {
        return Vec::new();
    };
    let Ok(len) = file.metadata().map(|m| m.len()) else {
        return Vec::new();
    };

    let mut tail: Vec<u8> = Vec::new();
    let mut pos = len;
    while pos > 0 {
        let start = pos.saturating_sub(CHUNK as u64);
        let mut chunk = vec![0u8; (pos - start) as usize];
        if file.seek(SeekFrom::Start(start)).is_err() || file.read_exact(&mut chunk).is_err() {
            break;
        }
        chunk.extend_from_slice(&tail);
        tail = chunk;
        pos = start;
        if tail.iter().filter(|&&b| b == b'\n').count() > limit {
            break;
        }
    }

    let text = String::from_utf8_lossy(&tail);
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_string())
        .collect();
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    if pos > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    let skip = lines.len().saturating_sub(limit);
    lines.drain(..skip);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("hestia-tail-{}-{name}", std::process::id()));
        let mut f = File::create(&path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        path
    }

    #[test]
    fn reads_the_last_lines() {
        let path = temp_file("last", "one\ntwo\nthree\n");
        assert_eq!(read_last_lines(&path, 2), vec!["two", "three"]);
        assert_eq!(read_last_lines(&path, 10), vec!["one", "two", "three"]);
        assert!(read_last_lines(&path, 0).is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_reads_empty() {
        assert!(read_last_lines(Path::new("/nonexistent/hestia.log"), 5).is_empty());
    }

    #[test]
    fn drain_emits_new_complete_lines_and_handles_truncation() {
        let path = temp_file("drain", "first\nsecond\npart");
        let mut offset = 0;
        let mut pending = Vec::new();
        let mut lines = Vec::new();
        drain(&path, &mut offset, &mut pending, |l| lines.push(l));
        assert_eq!(lines, vec!["first", "second"]);

        std::fs::write(&path, "fresh\n").unwrap();
        drain(&path, &mut offset, &mut pending, |l| lines.push(l));
        assert_eq!(lines, vec!["first", "second", "fresh"]);
        let _ = std::fs::remove_file(&path);
    }
}
