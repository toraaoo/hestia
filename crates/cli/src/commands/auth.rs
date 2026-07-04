//! `hestia auth …` — Microsoft/Minecraft sign-in.
//!
//! Accounts and the Xbox signing chain are not implemented yet, so these
//! commands report that the daemon has no accounts subsystem.

use anyhow::{bail, Result};
use clap::Subcommand;

#[derive(Subcommand)]
pub enum AuthCmd {
    /// Sign in with a Microsoft account
    Login {
        #[arg(long, help = "Use the browser-redirect (sisu) flow")]
        sisu: bool,
    },
    /// Signed-in accounts
    List,
    /// Sign out and forget the stored tokens
    Logout { account: String },
}

pub async fn run(_cmd: AuthCmd) -> Result<()> {
    bail!("account sign-in is not available in this build yet")
}
