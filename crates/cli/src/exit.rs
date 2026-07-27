//! The CLI's exit-code contract.
//!
//! A state query has two honest answers — "the thing is running" and "the thing
//! is not" — and a shell can only read one of them from the exit code. Printing
//! `stopped` and exiting 0 conflates *answering* with *affirming*, so
//! `if hestia daemon status; then …` is true whether or not the daemon is up.
//! Exiting 1 instead would be worse: it collapses "not running" into "the query
//! failed", which is the one distinction a script actually needs.
//!
//! So the vocabulary is systemd's, which exists for exactly this question:
//!
//! | code | meaning                                                      |
//! |------|--------------------------------------------------------------|
//! | 0    | the command did what was asked; a state query found it active |
//! | 3    | the command answered, and the subject is **not** running      |
//! | 1    | the command failed (no daemon, bad input, operation error)    |
//! | 2    | usage error — clap's own convention                           |
//!
//! Only verbs that assert whether one subject is running use 3: `daemon status`
//! and `server <name> status`. Verbs that *describe* rather than assert — `info`,
//! `sync status`, every list — always exit 0, because "inactive" is not a claim
//! they make.

use std::process::ExitCode;

/// A command's answer, as the shell will read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    /// The command did what was asked. For a state query: the subject is running.
    Active,
    /// The query was answered and the subject is not running. Never an error —
    /// the command worked.
    Inactive,
}

impl ExitStatus {
    /// The state query's answer for a subject that may or may not be running.
    pub fn running(running: bool) -> Self {
        match running {
            true => ExitStatus::Active,
            false => ExitStatus::Inactive,
        }
    }

    pub fn code(self) -> ExitCode {
        match self {
            ExitStatus::Active => ExitCode::SUCCESS,
            ExitStatus::Inactive => ExitCode::from(3),
        }
    }
}
