mod secrets;
mod utils;

use crate::secrets::EditCommand;
use anyhow::Result;
use clap::Clap;

#[derive(Clap)]
struct SecretStore {
    #[clap(subcommand)]
    cmd: SubCommands,
}

#[derive(Clap)]
enum SubCommands {
    /// Edit a secret interactively
    Edit(EditCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: SecretStore = SecretStore::parse();
    let manager = secrets::Manager::new();
    // manager.list().await;
    match opt.cmd {
        SubCommands::Edit(cmd) => manager.edit(cmd).await?,
    }
    Ok(())
}
