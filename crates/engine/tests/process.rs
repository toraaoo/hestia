//! Stopping a supervised process stops what it spawned.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::time::Duration;

use engine::ProcessSupervisor;
use proto::process::{LogSource, ProcessSpec, ProcessState, RestartPolicy};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "hestia-process-test-{}-{}",
        tag,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    base
}

fn alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

async fn wait_for(mut ready: impl FnMut() -> bool) -> bool {
    for _ in 0..100 {
        if ready() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[tokio::test]
async fn stopping_a_process_stops_its_children() {
    let dir = temp_dir("tree");
    let supervisor = ProcessSupervisor::new(dir.join("processes"));
    let child_pid_file = dir.join("child.pid");

    supervisor
        .start(ProcessSpec {
            id: "tree".into(),
            program: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                format!(
                    "sleep 300 & echo $! > {} ; wait",
                    child_pid_file.to_string_lossy()
                ),
            ],
            cwd: Some(dir.clone()),
            env: BTreeMap::new(),
            restart: RestartPolicy::Never,
            log: LogSource::Capture,
        })
        .await
        .expect("the process starts");

    assert!(
        wait_for(|| child_pid_file.is_file()).await,
        "the child records its pid"
    );
    let child: u32 = std::fs::read_to_string(&child_pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(alive(child), "the child is running before the stop");

    assert!(supervisor.stop("tree"), "the process is known");
    assert!(
        wait_for(|| supervisor
            .status("tree")
            .is_some_and(|p| p.state != ProcessState::Running))
        .await,
        "the process reaches a terminal state"
    );
    assert!(
        wait_for(|| !alive(child)).await,
        "the child goes with its parent — a stop reaches the whole tree"
    );
}
