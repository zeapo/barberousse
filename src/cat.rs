use anyhow::*;
use clap::Clap;
use rusoto_secretsmanager::{GetSecretValueRequest, SecretsManager};

use crate::secrets::Manager;
use crate::utils::{format_convert, pretty_print, ContentFormat};

#[derive(Clap)]
pub struct CatCommand {
    /// The id of the secret to edit
    secret_id: String,

    /// The format of the secret's remote storage
    #[clap(arg_enum, short = "s", long = "secret-format", default_value = "json")]
    secret_format: ContentFormat,

    /// The format used to print the secret, if the secret's format is `text`, this will be ignored
    /// and defaults to `text` too
    #[clap(arg_enum, short = "e", long = "print-format", default_value = "yaml")]
    print_format: ContentFormat,
}

impl Manager {
    pub async fn cat(&self, cmd: CatCommand) -> Result<()> {
        // deconstruct this little bad boy
        let CatCommand {
            secret_id,
            secret_format,
            print_format,
        } = cmd;

        let res = self
            .client
            .get_secret_value(GetSecretValueRequest {
                secret_id: secret_id.clone(),
                ..GetSecretValueRequest::default()
            })
            .await?;

        let remote_content = res
            .secret_string
            .as_ref()
            .expect("The secret_id is required");

        let formatted_content = format_convert(remote_content, &secret_format, &print_format)?;
        pretty_print(formatted_content, print_format)?;

        Ok(())
    }
}
