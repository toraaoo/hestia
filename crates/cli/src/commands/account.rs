//! `hestia account …` — Microsoft/Minecraft sign-in and account switching.

use std::io;
use std::process::Stdio;

use anyhow::{bail, Result};
use clap::Subcommand;
use client::proto::accounts::{Account, LoginMethod};
use client::Client;

use crate::ui::{self, Spinner, View};

#[derive(Subcommand)]
pub enum AccountCmd {
    /// Sign in to a Microsoft account
    Login {
        #[arg(
            long,
            help = "Sign in through the browser-redirect (sisu) flow instead of a device code"
        )]
        sisu: bool,
    },
    /// Signed-in accounts (* marks the one launches use)
    #[command(visible_alias = "ls")]
    List,
    /// Pick the account launches use; prompts when omitted
    #[command(visible_alias = "use")]
    Switch {
        /// Account name or uuid
        account: Option<String>,
    },
    /// Sign out of an account and forget its tokens
    Logout {
        /// Account name or uuid
        account: String,
    },
}

pub async fn run(cmd: AccountCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        AccountCmd::Login { sisu } => {
            let account = if sisu {
                sisu_login(&client).await?
            } else {
                device_code_login(&client).await?
            };
            ui::show(View::line(format!(
                "Signed in as {} ({})",
                account.name, account.uuid
            )))?;
        }
        AccountCmd::List => {
            let listing = client.accounts().list().await?;
            if listing.accounts.is_empty() {
                return ui::show(View::note("no accounts signed in (hestia account login)"));
            }
            let rows = listing
                .accounts
                .iter()
                .map(|a| {
                    let marker = if a.uuid == listing.default_uuid {
                        "*"
                    } else {
                        ""
                    };
                    vec![marker.to_string(), a.name.clone(), a.uuid.clone()]
                })
                .collect();
            ui::show(View::table("accounts", ["", "NAME", "UUID"], rows))?;
        }
        AccountCmd::Switch { account } => {
            let reference = match account {
                Some(reference) => reference,
                None => pick_account(&client).await?,
            };
            let switched = client.accounts().switch(&reference).await?;
            ui::show(View::line(format!(
                "launches now use {} ({})",
                switched.name, switched.uuid
            )))?;
        }
        AccountCmd::Logout { account } => {
            client.accounts().remove(&account).await?;
            ui::show(View::line(format!("Signed out {account}")))?;
        }
    }
    Ok(())
}

/// Interactive account picker for a `switch` without an argument.
async fn pick_account(client: &Client) -> Result<String> {
    let listing = client.accounts().list().await?;
    if listing.accounts.is_empty() {
        bail!("no accounts signed in (hestia account login)");
    }
    let labels: Vec<String> = listing
        .accounts
        .iter()
        .map(|a| {
            if a.uuid == listing.default_uuid {
                format!("{} (current)", a.name)
            } else {
                a.name.clone()
            }
        })
        .collect();
    let index = ui::select("switch launches to", &labels)?;
    Ok(listing.accounts[index].uuid.clone())
}

async fn device_code_login(client: &Client) -> Result<Account> {
    let flow = client
        .accounts()
        .begin_login(LoginMethod::DeviceCode)
        .await?;
    ui::show(View::line(format!(
        "To sign in, open\n\n  {}\n\nand enter the code\n\n  {}",
        flow.verification_uri, flow.user_code
    )))?;
    wait_for_enter("Press Enter to open your browser... ");
    open_browser(&flow.verification_uri);
    let _spinner = Spinner::start("waiting for you to finish in the browser…");
    Ok(client.accounts().complete_login(&flow.id, "").await?)
}

async fn sisu_login(client: &Client) -> Result<Account> {
    let flow = client.accounts().begin_login(LoginMethod::Sisu).await?;
    ui::show(View::line(format!(
        "Open this URL in your browser and sign in:\n\n  {}",
        flow.url
    )))?;
    wait_for_enter("Press Enter to open your browser... ");
    open_browser(&flow.url);
    ui::prompt(
        "You'll land on a blank page — paste its full address (or just the\ncode) here, then press Enter:\n> ",
    );

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let code = extract_code(&input);
    if code.is_empty() {
        bail!("no authorization code was pasted");
    }
    let _spinner = Spinner::start("signing in…");
    Ok(client.accounts().complete_login(&flow.id, &code).await?)
}

fn wait_for_enter(message: &str) {
    ui::prompt(message);
    let mut discard = String::new();
    let _ = io::stdin().read_line(&mut discard);
}

fn open_browser(url: &str) {
    if !url.starts_with("https://") || url.contains(['"', '\'']) {
        return;
    }
    let mut command = if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    } else if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    } else {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    let _ = command.stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

/// Pull the OAuth `code` out of a pasted redirect URL (or accept a bare code).
fn extract_code(pasted: &str) -> String {
    let input = pasted.trim();
    let Some(marker) = input.find("code=") else {
        return input.to_string();
    };
    let rest = &input[marker + 5..];
    let end = rest.find('&').unwrap_or(rest.len());
    url_decode(&rest[..end])
}

fn url_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(&value[i + 1..i + 3], 16) {
                    out.push(byte as char);
                    i += 3;
                    continue;
                }
                out.push('%');
                i += 1;
            }
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b => {
                out.push(b as char);
                i += 1;
            }
        }
    }
    out
}
