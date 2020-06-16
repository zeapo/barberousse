use anyhow::Result;
use rusoto_core;
use rusoto_secretsmanager::SecretsManager;
use rusoto_secretsmanager::{GetSecretValueRequest, PutSecretValueRequest, SecretsManagerClient};
use structopt::clap::arg_enum;
use structopt::StructOpt;

use serde::Deserialize;
use std::error::Error;
use std::io::{BufRead, Read, Seek, SeekFrom, Write};
use std::ops::Deref;
use std::str::FromStr;
use uuid::Uuid;

/// Manages the read/write of secrets
///

///
pub struct Manager {
    client: SecretsManagerClient,
}

arg_enum! {
    #[derive(Debug)]
    enum Format {
        json,
        yaml,
        text,
    }
}

#[derive(StructOpt)]
pub struct EditCommand {
    secret_id: String,
    /// The format of the secret's remote storage
    #[structopt(short = "s", long = "secret-format", default_value = "json")]
    secret_format: Format,
    /// The format used to edit the secret, if the secret's format is `text`, this will be ignored
    /// and defaults to `text` too
    #[structopt(short = "e", long = "edit-format", default_value = "yaml")]
    edit_format: Format,
}

/// Takes a string in [source_format] and outputs a string in [destination_format]
fn format_convert(
    content: &String,
    source_format: &Format,
    destination_format: &Format,
) -> Result<String> {
    Ok(match source_format {
        Format::json => {
            let json: serde_json::Value = serde_json::from_str(content)?;

            match destination_format {
                Format::json => serde_json::to_string_pretty(&json)?,
                Format::yaml => serde_yaml::to_string(&json)?,
                Format::text => String::from(content),
            }
        }
        Format::yaml => {
            let yaml: serde_yaml::Value = serde_yaml::from_str(content)
                .map_err(|e| anyhow::Error::new(e).context("Unable to parse YAML".to_string()))?;

            match destination_format {
                Format::json => serde_json::to_string_pretty(&yaml)?,
                Format::yaml => serde_yaml::to_string(&yaml)?,
                Format::text => String::from(content),
            }
        }
        Format::text => String::from(content),
    })
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

        let content = res.secret_string.as_ref().unwrap();

        let remote_content: String = format_convert(content, &secret_format, &edit_format)?;

        // write the yaml to content
        write!(tf, "{}", remote_content)?;

        // try to edit this secret, until we succeed , or that the
        let content: Option<String> = loop {
            let vim_cmd = format!(
                "vim {}",
                tf.as_ref()
                    .to_owned()
                    .to_str()
                    .expect("Unable to handle temp file... this should not happen")
            );

            let exit = std::process::Command::new("/usr/bin/sh")
                .arg("-c")
                .arg(vim_cmd)
                .spawn()
                .expect("failed")
                .wait()?;

            // read the file back
            tf.seek(SeekFrom::Start(0))?;
            let mut content = String::new();
            tf.read_to_string(&mut content)?;

            // convert the content back to its original format
            let edited = format_convert(&content, &edit_format, &secret_format);

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
        if let Some(content) = content {
            self.client
                .put_secret_value(PutSecretValueRequest {
                    secret_id,
                    secret_string: Some(content),
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
