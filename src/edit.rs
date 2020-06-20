use anyhow::*;
use clap::Clap;
use rusoto_secretsmanager::{GetSecretValueRequest, PutSecretValueRequest, SecretsManager};
use uuid::Uuid;

use crate::secrets::Manager;
use crate::utils::ContentFormat;

#[derive(Clap)]
pub struct EditCommand {
    /// The id of the secret to edit
    secret_id: String,

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
    pub async fn edit(&self, cmd: EditCommand) -> Result<()> {
        // deconstruct this little bad boy
        let EditCommand {
            secret_id,
            secret_format,
            edit_format,
            editor,
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

        let edited_content =
            crate::editor::edit_content(editor, &remote_content, secret_format, edit_format)?;

        // if the content was modified correctly
        if edited_content.ne(remote_content) {
            self.client
                .put_secret_value(PutSecretValueRequest {
                    secret_id,
                    secret_string: Some(edited_content),
                    client_request_token: Some(Uuid::new_v4().to_string()),
                    ..PutSecretValueRequest::default()
                })
                .await?;
        } else {
            // check if the file changed, otherwise no need to create a new version
            return Err(anyhow!(
                "Aborting save due to matching remote and edited secrets"
            ));
        }

        Ok(())
    }
}
