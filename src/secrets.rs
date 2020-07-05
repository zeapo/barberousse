use std::env;
use std::str::FromStr;

use anyhow::Result;
use anyhow::*;
use clap::Clap;
use rusoto_core;
use rusoto_secretsmanager::{
    CreateSecretRequest, GetSecretValueError, GetSecretValueRequest, PutSecretValueRequest,
    SecretsManager, SecretsManagerClient,
};
use uuid::Uuid;

use async_trait::async_trait;

use crate::utils::ContentFormat;
use crate::utils::{format_convert, pretty_print};
use rusoto_core::RusotoError;

#[derive(Clap)]
pub struct CatCommand {
    /// The id of the secret to print
    secret_id: String,

    /// The format of the secret's remote storage
    #[clap(arg_enum, short = "s", long = "secret-format", default_value = "json")]
    secret_format: ContentFormat,

    /// The format used to print the secret, if the secret's format is `text`, this will be ignored
    /// and defaults to `text` too
    #[clap(arg_enum, short = "e", long = "print-format", default_value = "yaml")]
    print_format: ContentFormat,
}

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

    /// Use a different region for the target secret
    #[clap(long = "target-region")]
    target_region: Option<String>,
}

#[async_trait]
pub trait SecretsManagerClientExt {
    fn new_client(profile: Option<String>, region: Option<String>) -> Result<SecretsManagerClient>;
    async fn cat_secret(&self, cmd: CatCommand) -> Result<()>;
    async fn edit_secret(&self, cmd: EditCommand) -> Result<()>;
    async fn copy_secret(&self, cmd: CopyCommand, profile: Option<String>) -> Result<()>;
}

#[async_trait]
impl SecretsManagerClientExt for SecretsManagerClient {
    /// Create a new manager client. Overrides the default `profile` and `region`
    /// if they are provided.
    fn new_client(profile: Option<String>, region: Option<String>) -> Result<SecretsManagerClient> {
        // FIXME use the ProfileProvider::with_default_credentials(profile) once it's merged
        //       in https://github.com/rusoto/rusoto/pull/1776
        //       For the moment rely on a hackish env variable change
        if let Some(profile) = profile {
            env::set_var("AWS_PROFILE", profile);
        }

        let region = match region {
            Some(r) => rusoto_core::Region::from_str(&r)?,
            None => rusoto_core::Region::default(),
        };

        Ok(SecretsManagerClient::new(region))
    }

    /// Print the content of a secret
    async fn cat_secret(&self, cmd: CatCommand) -> Result<()> {
        // deconstruct this little bad boy
        let CatCommand {
            secret_id,
            secret_format,
            print_format,
        } = cmd;

        let res = self
            .get_secret_value(GetSecretValueRequest {
                secret_id: secret_id.clone(),
                ..GetSecretValueRequest::default()
            })
            .await?;
        //
        let remote_content = res
            .secret_string
            .as_ref()
            .expect("The secret_id is required");
        //
        let formatted_content = format_convert(remote_content, &secret_format, &print_format)?;
        pretty_print(formatted_content, print_format)?;

        Ok(())
    }

    /// Edit the content of a secret
    async fn edit_secret(&self, cmd: EditCommand) -> Result<()> {
        // deconstruct this little bad boy
        let EditCommand {
            secret_id,
            secret_format,
            edit_format,
            editor,
        } = cmd;

        let res = self
            .get_secret_value(GetSecretValueRequest {
                secret_id: secret_id.clone(),
                ..GetSecretValueRequest::default()
            })
            .await;

        let res = match res {
            Ok(r) => Ok(Some(r)),
            Err(RusotoError::Service(GetSecretValueError::ResourceNotFound(_))) => Ok(None),
            Err(e) => Err(e),
        }?;

        let (remote_content, to_create) = match res {
            Some(r) => (r.secret_string.expect("The secret_id is required"), false),
            None => ("{\"\": \"\"}".to_string(), true),
        };

        let edited_content = crate::editor::edit_content(
            editor,
            &remote_content.to_string(),
            secret_format,
            edit_format,
        )?;

        // if the content was modified correctly
        if edited_content.ne(&remote_content) {
            if to_create {
                self.create_secret(CreateSecretRequest {
                    name: secret_id,
                    secret_string: Some(edited_content),
                    client_request_token: Some(Uuid::new_v4().to_string()),
                    ..CreateSecretRequest::default()
                })
                .await?;
            } else {
                self.put_secret_value(PutSecretValueRequest {
                    secret_id,
                    secret_string: Some(edited_content),
                    client_request_token: Some(Uuid::new_v4().to_string()),
                    ..PutSecretValueRequest::default()
                })
                .await?;
            }
        } else {
            // check if the file changed, otherwise no need to create a new version
            return Err(anyhow!(
                "Aborting save due to matching remote and edited secrets"
            ));
        }

        Ok(())
    }

    /// Copy a secret to another secret
    async fn copy_secret(&self, cmd: CopyCommand, profile: Option<String>) -> Result<()> {
        // deconstruct this little bad boy
        let CopyCommand {
            secret_id,
            target_id,
            secret_format,
            edit_format,
            editor,
            target_region,
        } = cmd;

        let res = self
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

        // create a new client pointing to the new region
        let target_client = if target_region != None {
            SecretsManagerClient::new_client(profile, target_region)?
        } else {
            self.clone()
        };

        // if the content was modified correctly
        target_client
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
