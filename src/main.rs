#[macro_use] extern crate shell;
use anyhow::Result;
use clap::Clap;
use rusoto_secretsmanager::SecretsManagerClient;

use crate::secrets::*;

mod editor;
mod secrets;
mod utils;

#[derive(Clap)]
struct SecretStore {
    /// Use a specific aws profile, overrides config and env settings
    #[clap(short = "P", long = "profile", global = true)]
    profile: Option<String>,

    /// The region where the secret is, overrides config and env settings
    #[clap(short = "R", long = "region", global = true)]
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
    let manager = SecretsManagerClient::new_client(opt.profile.clone(), opt.region)?;

    // manager.list().await;
    match opt.cmd {
        SubCommands::Edit(cmd) => manager.edit_secret(cmd).await?,
        SubCommands::Cat(cmd) => manager.cat_secret(cmd).await?,
        SubCommands::Copy(cmd) => manager.copy_secret(cmd, opt.profile).await?,
    }
    Ok(())
}
