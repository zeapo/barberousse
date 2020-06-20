use anyhow::*;
use clap::Clap;
use rusoto_secretsmanager::{CreateSecretRequest, GetSecretValueRequest, SecretsManager};
use uuid::Uuid;

use crate::secrets::Manager;
use crate::utils::ContentFormat;

#[derive(Clap)]
pub struct CopyCommand {
    /// The id of the secret to copy
    secret_id: String,

    /// The id of the secret to create
    target_id: String,

    /// The format of the secret's remote storage
    #[clap(arg_enum, short = "s", long = "secret-format", default_value = "json")]
    secret_format: ContentFormat,

    /// The format used to edit the secret, if the secret's format is `text`, this will be ignored
    /// and defaults to `text` too
    #[clap(arg_enum, short = "e", long = "edit-format", default_value = "yaml")]
    edit_format: ContentFormat,

    /// Override the default editor, $EDITOR, used for editing the secret
    #[clap(long = "editor")]
    editor: Option<String>,
}

impl Manager {
    pub async fn copy(&self, cmd: CopyCommand) -> Result<()> {
        // deconstruct this little bad boy
        let CopyCommand {
            secret_id,
            target_id,
            secret_format,
            edit_format,
            editor,
        } = cmd;

        //
        if target_id.eq(&secret_id) {
            // TODO this check should be different on introduction of cross-region & account
            return Err(anyhow!(
                "Source secret_id and target can't be equal on the same account and region"
            ));
        }

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

        let edited_content =
            crate::editor::edit_content(editor, &remote_content, secret_format, edit_format)?;

        // TODO prompt the user that the secrets are the same and that it's not a good pratice

        // if the content was modified correctly
        self.client
            .create_secret(CreateSecretRequest {
                name: target_id,
                secret_string: Some(edited_content),
                client_request_token: Some(Uuid::new_v4().to_string()),
                ..CreateSecretRequest::default()
            })
            .await?;

        Ok(())
    }
}
