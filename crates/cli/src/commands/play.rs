//! `hestia play` — the happy path: launch an instance. With no argument the
//! sole instance launches directly; several prompt an interactive pick.
//!
//! `--world` / `--server` skip the menus and join on start (Quick Play), the
//! same targets the entry-first `instance <name> world|server play` verbs take.

use anyhow::{bail, Result};

use crate::commands::instance::{self, multiplayer};
use crate::ui;

pub async fn run(
    reference: Option<String>,
    account: Option<String>,
    new_session: bool,
    detach: bool,
    world: Option<String>,
    server: Option<String>,
) -> Result<()> {
    let client = super::connect().await?;
    let reference = match reference {
        Some(reference) => reference,
        None => pick_instance(&client).await?,
    };
    instance::launch(
        &client,
        &reference,
        account.as_deref().unwrap_or_default(),
        new_session,
        detach,
        multiplayer::target(world, server),
    )
    .await
}

async fn pick_instance(client: &client::Client) -> Result<String> {
    let instances = client.instance().list().await?;
    match instances.len() {
        0 => bail!("no instances yet (hestia instance create)"),
        1 => Ok(instances[0].name.clone()),
        _ => {
            let labels: Vec<String> = instances
                .iter()
                .map(|i| format!("{} ({} {})", i.name, i.flavor, i.game_version))
                .collect();
            let index = ui::select("what do you want to play?", &labels)?;
            Ok(instances[index].name.clone())
        }
    }
}
