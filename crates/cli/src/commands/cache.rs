//! `hestia cache …` — the checksummed download cache.

use anyhow::Result;
use clap::Subcommand;

use crate::output::{human_bytes, print_table};

#[derive(Subcommand)]
pub enum CacheCmd {
    /// Location, entry count, and size
    Info,
    /// Cached blobs by checksum
    List,
    /// Remove everything, reporting what was freed
    Clear,
}

pub async fn run(cmd: CacheCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        CacheCmd::Info => {
            let info = client.cache().info().await?;
            println!("location: {}", info.path.display());
            println!("entries:  {}", info.usage.entries);
            println!("size:     {}", human_bytes(info.usage.bytes));
        }
        CacheCmd::List => {
            let entries = client.cache().list().await?;
            if entries.is_empty() {
                println!("cache is empty");
                return Ok(());
            }
            let rows = entries
                .iter()
                .map(|e| {
                    vec![
                        format!("{}:{}", e.checksum.algorithm.as_str(), e.checksum.hex),
                        human_bytes(e.size),
                    ]
                })
                .collect::<Vec<_>>();
            print_table(&["CHECKSUM", "SIZE"], &rows);
        }
        CacheCmd::Clear => {
            let freed = client.cache().clear().await?;
            println!(
                "freed {} across {} entries",
                human_bytes(freed.bytes),
                freed.entries
            );
        }
    }
    Ok(())
}
