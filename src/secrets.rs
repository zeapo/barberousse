use anyhow::{anyhow, Error, Result};
use clap::Clap;
use rusoto_core;
use rusoto_secretsmanager::SecretsManager;
use rusoto_secretsmanager::{GetSecretValueRequest, PutSecretValueRequest, SecretsManagerClient};

use crate::utils::{format_convert, ContentFormat};
use std::process::ExitStatus;
use std::{
    env::var,
    io::{Read, Seek, SeekFrom, Write},
    process::Command,
};
use uuid::Uuid;

/// Manages the read/write of secrets
///

///
pub struct Manager {
    client: SecretsManagerClient,
}

#[derive(Clap)]
#[clap(version = "0.0.1", author = "zeapo")]
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
    pub fn new() -> Manager {
        Manager {
            client: SecretsManagerClient::new(rusoto_core::Region::default()),
        }
    }

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

        // FIXME this is not secure as the tempfile will be visible in /tmp
        //  would be better idea to create a folder in in /dev/shm with a file mode 600
        //  so that only the user can edit/see it, then put the file in it
        let mut tf = tempfile::NamedTempFile::new()?;

        let remote_content = res
            .secret_string
            .as_ref()
            .expect("The secret_id is required");

        let formated_content: String =
            format_convert(remote_content, &secret_format, &edit_format)?;

        // write the yaml to content
        write!(tf, "{}", formated_content)?;

        // try to edit this secret, until we succeed , or that the
        let edited_content: Option<String> = loop {
            // Open the editor \o/
            open_editor(
                editor.clone(),
                tf.as_ref()
                    .to_owned()
                    .to_str()
                    .expect("Unable to handle temp file... this should not happen"),
            )?;

            // read the file back
            tf.seek(SeekFrom::Start(0))?;
            let mut saved_content = String::new();
            tf.read_to_string(&mut saved_content)?;

            // convert the content back to its original format
            let edited = format_convert(&saved_content, &edit_format, &secret_format);

            match edited {
                Ok(content) => {
                    break Some(content);
                }
                Err(e) => {
                    // TODO add a yes/no/ignore question to see if we continue, discard, ignore and save as text
                    eprintln!("{:?}", e);
                    let decision = promptly::prompt_default("Do you want to edit again?", true)?;
                    if !decision {
                        break None;
                    }
                }
            };
        };

        // if the content was modified correctly
        if let Some(edited_content) = edited_content {
            // TODO check if the file changed, otherwise no need to create a new version
            if edited_content.eq(remote_content) {
                return Err(anyhow!(
                    "Aborting save due to matching remote and edited secrets"
                ));
            }

            self.client
                .put_secret_value(PutSecretValueRequest {
                    secret_id,
                    secret_string: Some(edited_content),
                    client_request_token: Some(Uuid::new_v4().to_string()),
                    ..PutSecretValueRequest::default()
                })
                .await?;
        } else {
            println!("Edit was discarded.")
        }

        // this is not really needed
        tf.close()?;
        Ok(())
    }
}

/// Opens the editor to edit a specific file
fn open_editor(editor: Option<String>, path: &str) -> Result<ExitStatus> {
    // Open the editor \o/
    let editor = editor.unwrap_or_else(|| {
        // yeah, default to nano if nothing is available
        var("EDITOR").unwrap_or("nano".to_string())
    });

    let exit = Command::new(editor)
        .arg(path)
        .spawn()
        .map_err(|e| Error::new(e).context("Unable to launch editor".to_string()))?
        .wait()?;

    Ok(exit)
}
