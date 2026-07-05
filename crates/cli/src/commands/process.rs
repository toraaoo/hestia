//! `hestia process …` — launch and supervise processes through the daemon.

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use client::proto::process::{LogStream, ProcessSpec, ProcessState, RestartPolicy};

use crate::output::print_table;

#[derive(Subcommand)]
pub enum ProcessCmd {
    /// Launch a process as a child of the daemon
    Start {
        program: String,
        /// Arguments passed to the program (use `--` to separate them from flags)
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
        #[arg(long, help = "Working directory for the process")]
        cwd: Option<PathBuf>,
        #[arg(long, help = "Re-spawn the process if it exits non-zero")]
        restart_on_failure: bool,
        #[arg(long, help = "Stream output and block until the process exits")]
        wait: bool,
    },
    /// Tracked processes and their state
    List,
    /// State of one process
    Status { id: String },
    /// Terminate a process
    Stop { id: String },
    /// Captured output of a process
    Logs {
        id: String,
        #[arg(long, help = "Only the last N lines")]
        tail: Option<usize>,
    },
}

pub async fn run(cmd: ProcessCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        ProcessCmd::Start {
            program,
            args,
            cwd,
            restart_on_failure,
            wait,
        } => {
            let spec = ProcessSpec {
                program: program.clone(),
                args,
                cwd,
                restart: if restart_on_failure {
                    RestartPolicy::OnFailure
                } else {
                    RestartPolicy::Never
                },
                ..Default::default()
            };
            if wait {
                let exit = client
                    .process()
                    .run(spec, |line| match line.stream {
                        LogStream::Stdout => println!("{}", line.line),
                        LogStream::Stderr => eprintln!("{}", line.line),
                    })
                    .await?;
                match exit.exit_code {
                    Some(code) => println!("exited with code {code}"),
                    None => println!("terminated"),
                }
            } else {
                let started = client.process().start(spec).await?;
                println!("started {} (pid {})", started.id, started.pid);
            }
        }
        ProcessCmd::List => {
            let processes = client.process().list().await?;
            if processes.is_empty() {
                println!("no tracked processes");
                return Ok(());
            }
            let rows = processes
                .iter()
                .map(|p| {
                    vec![
                        p.id.clone(),
                        p.pid.to_string(),
                        describe_state(p.state, p.exit_code),
                        p.program.clone(),
                    ]
                })
                .collect::<Vec<_>>();
            print_table(&["ID", "PID", "STATE", "PROGRAM"], &rows);
        }
        ProcessCmd::Status { id } => {
            let p = client.process().status(&id).await?;
            println!("id:      {}", p.id);
            println!("pid:     {}", p.pid);
            println!("program: {}", p.program);
            if !p.args.is_empty() {
                println!("args:    {}", p.args.join(" "));
            }
            println!("state:   {}", describe_state(p.state, p.exit_code));
        }
        ProcessCmd::Stop { id } => {
            client.process().stop(&id).await?;
            println!("stopping {id}");
        }
        ProcessCmd::Logs { id, tail } => {
            let lines = client.process().logs(&id, tail).await?;
            for line in lines {
                match line.stream {
                    LogStream::Stdout => println!("{}", line.line),
                    LogStream::Stderr => eprintln!("{}", line.line),
                }
            }
        }
    }
    Ok(())
}

fn describe_state(state: ProcessState, exit_code: Option<i32>) -> String {
    match state {
        ProcessState::Running => "running".to_string(),
        ProcessState::Exited => match exit_code {
            Some(code) => format!("exited ({code})"),
            None => "exited".to_string(),
        },
        ProcessState::Killed => "killed".to_string(),
    }
}
