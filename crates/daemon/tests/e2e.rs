//! End-to-end test: spawn the real `hestiad` binary on an isolated socket and
//! data directory, then drive it through the client SDK exactly as a front-end
//! would. This exercises the full stack — transport, router, services, the
//! process supervisor — against a running daemon.
//!
//! Unix-only: it relies on a POSIX shell for the launched child and on the
//! domain-socket transport. Windows is validated through the win-VM flow.
#![cfg(unix)]

use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use client::proto::process::{ProcessSpec, ProcessState};
use client::Client;

/// A spawned daemon that is stopped and reaped on drop.
struct Daemon {
    child: Child,
    home: std::path::PathBuf,
    cleanup: bool,
}

impl Daemon {
    fn wait_exit(&mut self) {
        for _ in 0..100 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("daemon did not exit within 5s");
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if self.cleanup {
            let _ = std::fs::remove_dir_all(&self.home);
        }
    }
}

fn unique_dir() -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("hestia-e2e-{}-{}", std::process::id(), n));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

async fn spawn_daemon() -> (Daemon, Client) {
    spawn_daemon_at(unique_dir(), true).await
}

async fn spawn_daemon_at(home: std::path::PathBuf, cleanup: bool) -> (Daemon, Client) {
    let sock = home.join("hestiad.sock");
    let child = Command::new(env!("CARGO_BIN_EXE_hestiad"))
        .arg("serve")
        .env("HESTIA_SOCK", &sock)
        .env("HESTIA_HOME", &home)
        .env("HESTIA_NO_TRAY", "1")
        .spawn()
        .expect("spawn hestiad");

    // Wait for the daemon to bind and accept connections.
    let mut client = None;
    for _ in 0..100 {
        if let Ok(c) = Client::connect_to(&sock).await {
            if c.app().ping().await.is_ok() {
                client = Some(c);
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let client = client.expect("daemon did not become reachable within 5s");
    (
        Daemon {
            child,
            home,
            cleanup,
        },
        client,
    )
}

fn pid_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

#[tokio::test]
async fn daemon_serves_the_full_client_surface() {
    let (daemon, client) = spawn_daemon().await;

    // Identity.
    let info = client.app().info().await.expect("app.info");
    assert_eq!(info.name, "Hestia");

    // The reserved `home` key round-trips the isolated data directory.
    let home = client.config().get("home").await.expect("config.get home");
    let home = home
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    assert_eq!(
        std::path::Path::new(&home),
        daemon.home.as_path(),
        "the daemon should report the HESTIA_HOME data directory"
    );

    // An empty engine has no runtimes and an addressable cache.
    assert!(client.java().list().await.expect("java.list").is_empty());
    let cache = client.cache().info().await.expect("cache.info");
    assert!(cache.path.starts_with(&daemon.home));

    // Run a process to completion and capture its output and exit code.
    let captured = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = captured.clone();
    let exit = client
        .process()
        .run(
            ProcessSpec {
                program: "/bin/sh".into(),
                args: vec!["-c".into(), "echo hello; exit 3".into()],
                ..Default::default()
            },
            move |line| sink.lock().unwrap().push(line.line.clone()),
        )
        .await
        .expect("process.run");
    assert_eq!(exit.exit_code, Some(3));
    assert!(!exit.success);
    assert!(
        captured.lock().unwrap().iter().any(|l| l == "hello"),
        "captured output should contain the child's stdout line"
    );

    // Start a long-running process, observe it, then stop it.
    let started = client
        .process()
        .start(ProcessSpec {
            program: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "while true; do echo tick; sleep 1; done".into(),
            ],
            ..Default::default()
        })
        .await
        .expect("process.start");
    assert!(started.pid > 0);

    tokio::time::sleep(Duration::from_millis(1200)).await;
    let status = client
        .process()
        .status(&started.id)
        .await
        .expect("process.status");
    assert_eq!(status.state, ProcessState::Running);
    let logs = client
        .process()
        .logs(&started.id, None)
        .await
        .expect("process.logs");
    assert!(
        logs.iter().any(|l| l.line == "tick"),
        "buffered logs should hold output"
    );

    client
        .process()
        .stop(&started.id)
        .await
        .expect("process.stop");
    let mut killed = false;
    for _ in 0..40 {
        let s = client
            .process()
            .status(&started.id)
            .await
            .expect("process.status");
        if s.state == ProcessState::Killed {
            killed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(killed, "the process should report Killed after stop");

    client.daemon().stop(false).await.expect("daemon.stop");
    drop(daemon);
}

#[tokio::test]
async fn supervised_processes_survive_a_daemon_restart() {
    let home = unique_dir();
    let (mut daemon, client) = spawn_daemon_at(home.clone(), false).await;

    let started = client
        .process()
        .start(ProcessSpec {
            id: "e2e-survivor".into(),
            program: "/bin/sh".into(),
            args: vec!["-c".into(), "sleep 60".into()],
            ..Default::default()
        })
        .await
        .expect("process.start");
    let pid = started.pid;
    assert!(pid_alive(pid));

    client.daemon().stop(false).await.expect("daemon.stop");
    drop(client);
    daemon.wait_exit();
    assert!(
        pid_alive(pid),
        "the process should outlive the daemon that spawned it"
    );

    let (daemon2, client) = spawn_daemon_at(home.clone(), true).await;
    let status = client
        .process()
        .status("e2e-survivor")
        .await
        .expect("process.status after restart");
    assert_eq!(status.state, ProcessState::Running);
    assert_eq!(status.pid, pid, "the adopted process keeps its pid");

    client
        .process()
        .stop("e2e-survivor")
        .await
        .expect("process.stop");
    let mut stopped = false;
    for _ in 0..100 {
        let s = client.process().status("e2e-survivor").await.unwrap();
        if s.state == ProcessState::Killed {
            stopped = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(stopped, "an adopted process should still be stoppable");
    assert!(!pid_alive(pid));

    drop(daemon2);
    drop(daemon);
}

#[tokio::test]
async fn daemon_stop_all_takes_workloads_down() {
    let (mut daemon, client) = spawn_daemon().await;

    let started = client
        .process()
        .start(ProcessSpec {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), "sleep 60".into()],
            ..Default::default()
        })
        .await
        .expect("process.start");

    client.daemon().stop(true).await.expect("daemon.stop");
    drop(client);
    daemon.wait_exit();
    for _ in 0..50 {
        if !pid_alive(started.pid) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("stop --all should take the workload down with the daemon");
}
