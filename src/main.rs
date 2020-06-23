use anyhow::Result;
use clap::Clap;

use crate::cat::CatCommand;
use crate::copy::CopyCommand;
use crate::edit::EditCommand;

mod cat;
mod copy;
mod edit;
mod editor;
mod secrets;
mod utils;

#[derive(Clap)]
struct SecretStore {
    /// Use a specific aws profile, overrides config and env settings
    #[clap(long = "profile", global = true)]
    profile: Option<String>,

    /// The region where the secret is, overrides config and env settings
    #[clap(long = "region", global = true)]
    region: Option<String>,

    #[clap(subcommand)]
    cmd: SubCommands,
}

#[derive(Clap)]
enum SubCommands {
    /// Edit a secret interactively
    Edit(EditCommand),
    /// Cat a secret
    Cat(CatCommand),
    /// Cat a secret
    Copy(CopyCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: SecretStore = SecretStore::parse();
    let manager = secrets::Manager::new(opt.profile, opt.region)?;
    // manager.list().await;
    match opt.cmd {
        SubCommands::Edit(cmd) => manager.edit(cmd).await?,
        SubCommands::Cat(cmd) => manager.cat(cmd).await?,
        SubCommands::Copy(cmd) => manager.copy(cmd).await?,
    }
    Ok(())
}
