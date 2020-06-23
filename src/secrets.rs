use std::env;
use std::str::FromStr;

use anyhow::Result;
use rusoto_core;
use rusoto_secretsmanager::SecretsManagerClient;

/// Manages the read/write of secrets
///

///
pub struct Manager {
    pub client: SecretsManagerClient,
}

impl Manager {
    /// Create a new manager client. Overrides the default `profile` and `region`
    /// if they are provided.
    pub fn new(profile: Option<String>, region: Option<String>) -> Result<Manager> {
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

        Ok(Manager {
            client: SecretsManagerClient::new(region),
        })
    }
}
