//! `hestia auth …` — Microsoft/Minecraft sign-in.

use std::io::{self, Write};
use std::process::Stdio;

use anyhow::{bail, Result};
use clap::Subcommand;
use client::proto::accounts::{Account, LoginMethod};
use client::Client;

use crate::output::print_table;
use crate::ui::Spinner;

#[derive(Subcommand)]
pub enum AuthCmd {
    /// Sign in to a Microsoft account
    Login {
        #[arg(long, help = "Sign in through the browser-redirect (sisu) flow instead of a device code")]
        sisu: bool,
    },
    /// List signed-in accounts
    List,
    /// Sign out of an account and forget its tokens
    Logout { account: String },
}

pub async fn run(cmd: AuthCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        AuthCmd::Login { sisu } => {
            let account =
                if sisu { sisu_login(&client).await? } else { device_code_login(&client).await? };
            println!("Signed in as {} ({})", account.name, account.uuid);
        }
        AuthCmd::List => {
            let accounts = client.accounts().list().await?;
            let rows = accounts.iter().map(|a| vec![a.name.clone(), a.uuid.clone()]).collect::<Vec<_>>();
            print_table(&["NAME", "UUID"], &rows);
        }
        AuthCmd::Logout { account } => {
            client.accounts().remove(&account).await?;
            println!("Signed out {account}");
        }
    }
    Ok(())
}

async fn device_code_login(client: &Client) -> Result<Account> {
    let flow = client.accounts().begin_login(LoginMethod::DeviceCode).await?;
    println!(
        "\nTo sign in, open\n\n  {}\n\nand enter the code\n\n  {}\n",
        flow.verification_uri, flow.user_code
    );
    wait_for_enter("Press Enter to open your browser... ");
    open_browser(&flow.verification_uri);
    let _spinner = Spinner::start("waiting for you to finish in the browser…");
    Ok(client.accounts().complete_login(&flow.id, "").await?)
}

async fn sisu_login(client: &Client) -> Result<Account> {
    let flow = client.accounts().begin_login(LoginMethod::Sisu).await?;
    println!("Open this URL in your browser and sign in:\n\n  {}\n", flow.url);
    wait_for_enter("Press Enter to open your browser... ");
    open_browser(&flow.url);
    print!(
        "You'll land on a blank page — paste its full address (or just the\ncode) here, then press Enter:\n> "
    );
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let code = extract_code(&input);
    if code.is_empty() {
        bail!("no authorization code was pasted");
    }
    let _spinner = Spinner::start("signing in…");
    Ok(client.accounts().complete_login(&flow.id, &code).await?)
}

fn wait_for_enter(prompt: &str) {
    print!("{prompt}");
    io::stdout().flush().ok();
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
