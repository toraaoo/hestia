//! Process supervision: launches processes as children of the daemon, tracks
//! them in a table, streams their captured output as events, and applies a
//! restart policy. Reaping a child yields its exit code. A launched process is
//! run by the same user the daemon runs as (the socket is owner-authorized), so
//! this is no more privileged than the user spawning it directly.

use std::collections::{HashMap, VecDeque};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ipc::protocol::Event;
use proto::process::{
    LogStream, ProcessExitEvent, ProcessInfo, ProcessLogLine, ProcessOutputEvent, ProcessSpec,
    ProcessStartedEvent, ProcessState, RestartPolicy,
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Notify;

use super::event_hub::EventHub;

/// Cap on the per-process line buffer served by `process.logs`; live output is
/// always published as events, so this only bounds after-the-fact retrieval.
const MAX_LOG_LINES: usize = 2000;
/// How many times `OnFailure` re-spawns a process before giving up.
const MAX_RESTARTS: u32 = 3;

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn topic_event<E: proto::Topic + serde::Serialize>(event: &E) -> Event {
    Event {
        topic: E::TOPIC.to_string(),
        payload: serde_json::to_value(event).unwrap_or_default(),
    }
}

fn generate_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    format!("process-{}-{}", std::process::id(), n)
}

/// The shared, live state of one tracked process. Cloned as an `Arc` into the
/// supervision task and read back through the table by the query handlers.
struct Entry {
    info: Mutex<ProcessInfo>,
    logs: Mutex<VecDeque<ProcessLogLine>>,
    stopping: AtomicBool,
    stop_notify: Notify,
}

impl Entry {
    fn snapshot(&self) -> ProcessInfo {
        self.info.lock().unwrap().clone()
    }

    fn record_line(&self, line: ProcessLogLine) {
        let mut logs = self.logs.lock().unwrap();
        if logs.len() == MAX_LOG_LINES {
            logs.pop_front();
        }
        logs.push_back(line);
    }
}

pub struct ProcessSupervisor {
    hub: Arc<EventHub>,
    table: Mutex<HashMap<String, Arc<Entry>>>,
}

impl ProcessSupervisor {
    pub fn new(hub: Arc<EventHub>) -> Self {
        ProcessSupervisor {
            hub,
            table: Mutex::new(HashMap::new()),
        }
    }

    /// Launch `spec` and begin supervising it. Returns the initial snapshot, or
    /// an error if the program is empty or fails to spawn (e.g. not found).
    pub async fn start(&self, mut spec: ProcessSpec) -> Result<ProcessInfo, StartError> {
        if spec.program.trim().is_empty() {
            return Err(StartError::EmptyProgram);
        }
        if spec.id.is_empty() {
            spec.id = generate_id();
        }

        let mut child = build_command(&spec).spawn().map_err(StartError::Spawn)?;
        let pid = child.id().unwrap_or(0);

        let entry = Arc::new(Entry {
            info: Mutex::new(ProcessInfo {
                id: spec.id.clone(),
                pid,
                program: spec.program.clone(),
                args: spec.args.clone(),
                state: ProcessState::Running,
                exit_code: None,
                started_unix: now_unix(),
            }),
            logs: Mutex::new(VecDeque::new()),
            stopping: AtomicBool::new(false),
            stop_notify: Notify::new(),
        });

        self.table
            .lock()
            .unwrap()
            .insert(spec.id.clone(), entry.clone());
        let snapshot = entry.snapshot();

        self.hub.publish(&topic_event(&ProcessStartedEvent {
            id: spec.id.clone(),
            pid,
        }));

        attach_readers(&mut child, &spec.id, &entry, &self.hub);
        let hub = self.hub.clone();
        tokio::spawn(async move {
            supervise(entry, child, spec, hub).await;
        });

        Ok(snapshot)
    }

    /// Signal a process to terminate. Returns false if no such id is tracked.
    pub fn stop(&self, id: &str) -> bool {
        let entry = self.table.lock().unwrap().get(id).cloned();
        match entry {
            Some(entry) => {
                entry.stopping.store(true, Ordering::SeqCst);
                // notify_one stores a permit, so a stop that races ahead of the
                // supervision task's wait is not lost.
                entry.stop_notify.notify_one();
                true
            }
            None => false,
        }
    }

    pub fn list(&self) -> Vec<ProcessInfo> {
        self.table
            .lock()
            .unwrap()
            .values()
            .map(|e| e.snapshot())
            .collect()
    }

    pub fn status(&self, id: &str) -> Option<ProcessInfo> {
        self.table.lock().unwrap().get(id).map(|e| e.snapshot())
    }

    pub fn logs(&self, id: &str, tail: Option<usize>) -> Option<Vec<ProcessLogLine>> {
        let table = self.table.lock().unwrap();
        let entry = table.get(id)?;
        let logs = entry.logs.lock().unwrap();
        let start = match tail {
            Some(n) => logs.len().saturating_sub(n),
            None => 0,
        };
        Some(logs.iter().skip(start).cloned().collect())
    }
}

/// A typed launch failure, mapped to a protocol error code at the service edge.
#[derive(Debug)]
pub enum StartError {
    EmptyProgram,
    Spawn(std::io::Error),
}

fn build_command(spec: &ProcessSpec) -> Command {
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(cwd) = &spec.cwd {
        cmd.current_dir(cwd);
    }
    for (key, value) in &spec.env {
        cmd.env(key, value);
    }
    cmd
}

/// Spawn a reader task per output stream: each line is buffered for `process.logs`
/// and published as a `process.output` event as it arrives.
fn attach_readers(child: &mut Child, id: &str, entry: &Arc<Entry>, hub: &Arc<EventHub>) {
    if let Some(stdout) = child.stdout.take() {
        spawn_reader(
            stdout,
            LogStream::Stdout,
            id.to_string(),
            entry.clone(),
            hub.clone(),
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_reader(
            stderr,
            LogStream::Stderr,
            id.to_string(),
            entry.clone(),
            hub.clone(),
        );
    }
}

fn spawn_reader<R>(reader: R, stream: LogStream, id: String, entry: Arc<Entry>, hub: Arc<EventHub>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let record = ProcessLogLine { stream, line };
            entry.record_line(record.clone());
            hub.publish(&topic_event(&ProcessOutputEvent {
                id: id.clone(),
                line: record,
            }));
        }
    });
}

async fn supervise(entry: Arc<Entry>, mut child: Child, spec: ProcessSpec, hub: Arc<EventHub>) {
    let mut attempts = 0u32;
    loop {
        let status = tokio::select! {
            r = child.wait() => r,
            _ = entry.stop_notify.notified() => {
                let _ = child.start_kill();
                child.wait().await
            }
        };

        let killed = entry.stopping.load(Ordering::SeqCst);
        let code = status.ok().and_then(|s| s.code());
        let success = !killed && code == Some(0);
        let state = if killed {
            ProcessState::Killed
        } else {
            ProcessState::Exited
        };

        {
            let mut info = entry.info.lock().unwrap();
            info.state = state;
            info.exit_code = code;
        }
        hub.publish(&topic_event(&ProcessExitEvent {
            id: spec.id.clone(),
            state,
            exit_code: code,
            success,
        }));

        let should_restart = !killed
            && spec.restart == RestartPolicy::OnFailure
            && !success
            && attempts < MAX_RESTARTS;
        if !should_restart {
            break;
        }

        attempts += 1;
        tokio::time::sleep(Duration::from_millis(500)).await;
        match build_command(&spec).spawn() {
            Ok(mut next) => {
                let pid = next.id().unwrap_or(0);
                {
                    let mut info = entry.info.lock().unwrap();
                    info.pid = pid;
                    info.state = ProcessState::Running;
                    info.exit_code = None;
                }
                hub.publish(&topic_event(&ProcessStartedEvent {
                    id: spec.id.clone(),
                    pid,
                }));
                attach_readers(&mut next, &spec.id, &entry, &hub);
                child = next;
            }
            Err(_) => break,
        }
    }
}
