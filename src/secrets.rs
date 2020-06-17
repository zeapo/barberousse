use rusoto_core;
use rusoto_secretsmanager::SecretsManagerClient;

/// Manages the read/write of secrets
///

///
pub struct Manager {
    pub client: SecretsManagerClient,
}

impl Manager {
    pub fn new() -> Manager {
        Manager {
            client: SecretsManagerClient::new(rusoto_core::Region::default()),
        }
    }
}
