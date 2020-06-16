mod secrets;

use crate::secrets::EditCommand;
use anyhow::Result;
use structopt::StructOpt;

#[derive(StructOpt)]
enum SecretStore {
    Edit(EditCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: SecretStore = SecretStore::from_args();
    let manager = secrets::Manager::new();
    // manager.list().await;
    match opt {
        SecretStore::Edit(e) => manager.edit(e).await?,
    }
    Ok(())
}
