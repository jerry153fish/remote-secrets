use crate::aws_common::{aws_test_endpoint, get_aws_sdk_config, is_test_env};
use async_trait::async_trait;
use cached::proc_macro::cached;
use crd::{Backend, RemoteValue, SecretData};

use anyhow::Result;
use k8s_openapi::ByteString;
use std::collections::BTreeMap;
use std::time::Duration;

use utils::value::{get_secret_data, merge_secret_data};

pub struct SecretManager {
    data: Vec<SecretData>,
}

#[async_trait]
trait SecretsManagerStore {
    async fn get_secret(&self, name: &str) -> Result<String>;
}

struct AwsSecretsManagerStore {
    client: aws_sdk_secretsmanager::Client,
}

impl AwsSecretsManagerStore {
    async fn new() -> Result<Self> {
        let shared_config = get_aws_sdk_config().await?;
        Ok(Self {
            client: secretsmanager_client(&shared_config),
        })
    }
}

#[async_trait]
impl SecretsManagerStore for AwsSecretsManagerStore {
    async fn get_secret(&self, name: &str) -> Result<String> {
        let output = self
            .client
            .get_secret_value()
            .secret_id(name)
            .send()
            .await?;
        let result = output.secret_string().unwrap_or_default();
        Ok(result.to_string())
    }
}

impl SecretManager {
    async fn get_value_with_store<S: SecretsManagerStore + Send + Sync>(
        &self,
        store: &S,
    ) -> BTreeMap<String, ByteString> {
        let mut secrets = BTreeMap::new();

        for secret_data in &self.data {
            match store.get_secret(&secret_data.value).await {
                Ok(data) => {
                    let data = get_secret_data(secret_data, &data);
                    secrets = merge_secret_data(data, secrets);
                }
                Err(err) => {
                    log::error!("{err}");
                }
            }
        }

        secrets
    }
}

#[async_trait]
impl RemoteValue for SecretManager {
    fn from_backend(backend: &Backend) -> SecretManager {
        SecretManager {
            data: backend.data.clone(),
        }
    }

    async fn get_value(&self) -> BTreeMap<String, ByteString> {
        match AwsSecretsManagerStore::new().await {
            Ok(store) => self.get_value_with_store(&store).await,
            Err(err) => {
                log::error!("{err}");
                BTreeMap::new()
            }
        }
    }
}

/// get the secret manager client
pub fn secretsmanager_client(conf: &aws_types::SdkConfig) -> aws_sdk_secretsmanager::Client {
    let mut secretsmanager_config_builder = aws_sdk_secretsmanager::config::Builder::from(conf);
    if is_test_env() {
        log::info!(
            "Using mocked AWS endpoint for SecretsManager {}",
            aws_test_endpoint()
        );
        secretsmanager_config_builder =
            secretsmanager_config_builder.endpoint_url(aws_test_endpoint())
    }
    aws_sdk_secretsmanager::Client::from_conf(secretsmanager_config_builder.build())
}

/// get the data from the secret manager store by name
/// Will cache the result for 60s
#[cached(time = 60, result = true)]
pub async fn get_secretsmanager_parameter(name: String) -> Result<String> {
    let store = AwsSecretsManagerStore::new().await?;
    store.get_secret(&name).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use std::collections::{BTreeSet, HashMap};

    struct FakeSecretsManagerStore {
        values: HashMap<String, String>,
        failures: BTreeSet<String>,
    }

    #[async_trait]
    impl SecretsManagerStore for FakeSecretsManagerStore {
        async fn get_secret(&self, name: &str) -> Result<String> {
            if self.failures.contains(name) {
                return Err(anyhow!("missing secret: {name}"));
            }

            self.values
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow!("unexpected secret: {name}"))
        }
    }

    fn skip_without_mock_env() -> bool {
        if crate::aws_common::should_run_aws_integration_tests() {
            return false;
        }

        eprintln!("Skipping AWS integration test: TEST_ENV=true is required");
        true
    }

    #[tokio::test]
    async fn test_secret_manager_get_value_maps_plain_and_json_data() {
        let backend = Backend {
            backend: crd::BackendType::SecretManager,
            data: vec![
                SecretData {
                    value: "plain-secret".to_string(),
                    is_json_string: None,
                    remote_path: None,
                    key: Some("token".to_string()),
                    configuration_profile_id: None,
                    version_number: None,
                },
                SecretData {
                    value: "json-secret".to_string(),
                    is_json_string: Some(true),
                    remote_path: Some("service.password".to_string()),
                    key: Some("password".to_string()),
                    configuration_profile_id: None,
                    version_number: None,
                },
            ],
            pulumi_token: None,
        };

        let store = FakeSecretsManagerStore {
            values: HashMap::from([
                ("plain-secret".to_string(), "plain-value".to_string()),
                (
                    "json-secret".to_string(),
                    r#"{"service":{"password":"s3cr3t"}}"#.to_string(),
                ),
            ]),
            failures: BTreeSet::new(),
        };

        let result = SecretManager::from_backend(&backend)
            .get_value_with_store(&store)
            .await;

        assert_eq!(
            result.get("token"),
            Some(&ByteString(b"plain-value".to_vec()))
        );
        assert_eq!(
            result.get("password"),
            Some(&ByteString(b"s3cr3t".to_vec()))
        );
    }

    #[tokio::test]
    async fn test_secret_manager_get_value_skips_failed_secrets() {
        let backend = Backend {
            backend: crd::BackendType::SecretManager,
            data: vec![
                SecretData {
                    value: "present-secret".to_string(),
                    is_json_string: None,
                    remote_path: None,
                    key: Some("present".to_string()),
                    configuration_profile_id: None,
                    version_number: None,
                },
                SecretData {
                    value: "missing-secret".to_string(),
                    is_json_string: None,
                    remote_path: None,
                    key: Some("missing".to_string()),
                    configuration_profile_id: None,
                    version_number: None,
                },
            ],
            pulumi_token: None,
        };

        let store = FakeSecretsManagerStore {
            values: HashMap::from([("present-secret".to_string(), "ok".to_string())]),
            failures: BTreeSet::from(["missing-secret".to_string()]),
        };

        let result = SecretManager::from_backend(&backend)
            .get_value_with_store(&store)
            .await;

        assert_eq!(result.len(), 1);
        assert_eq!(result.get("present"), Some(&ByteString(b"ok".to_vec())));
    }

    #[tokio::test]
    async fn test_get_secretsmanager_parameter_mock_smoke() {
        if skip_without_mock_env() {
            return;
        }

        let result = get_secretsmanager_parameter("MyTestSecret".to_string())
            .await
            .unwrap();
        assert_eq!(result, "Vicd");
    }
}
