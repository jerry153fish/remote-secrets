use crate::aws_common::{aws_test_endpoint, get_aws_sdk_config, is_test_env};
use async_trait::async_trait;
use cached::proc_macro::cached;
use crd::{Backend, RemoteValue, SecretData};

use anyhow::{anyhow, Result};
use k8s_openapi::ByteString;
use std::collections::BTreeMap;
use std::time::Duration;

use utils::value::{get_secret_data, merge_secret_data};

pub struct SSM {
    data: Vec<SecretData>,
}

#[async_trait]
trait SsmStore {
    async fn get_parameter(&self, name: &str) -> Result<String>;
}

struct AwsSsmStore {
    client: aws_sdk_ssm::Client,
}

impl AwsSsmStore {
    async fn new() -> Result<Self> {
        let shared_config = get_aws_sdk_config().await?;
        Ok(Self {
            client: ssm_client(&shared_config),
        })
    }
}

#[async_trait]
impl SsmStore for AwsSsmStore {
    async fn get_parameter(&self, name: &str) -> Result<String> {
        let parameter = self.client.get_parameter().name(name).send().await?;
        let result = parameter
            .parameter()
            .ok_or_else(|| anyhow!("no parameter found"))?
            .value()
            .unwrap_or_default();

        Ok(result.to_string())
    }
}

impl SSM {
    async fn get_value_with_store<S: SsmStore + Send + Sync>(
        &self,
        store: &S,
    ) -> BTreeMap<String, ByteString> {
        let mut secrets = BTreeMap::new();

        for secret_data in &self.data {
            match store.get_parameter(&secret_data.value).await {
                Ok(value) => {
                    let data = get_secret_data(secret_data, &value);
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
impl RemoteValue for SSM {
    fn from_backend(backend: &Backend) -> SSM {
        SSM {
            data: backend.data.clone(),
        }
    }

    async fn get_value(&self) -> BTreeMap<String, ByteString> {
        match AwsSsmStore::new().await {
            Ok(store) => self.get_value_with_store(&store).await,
            Err(err) => {
                log::error!("{err}");
                BTreeMap::new()
            }
        }
    }
}

/// get the ssm client
pub fn ssm_client(conf: &aws_types::SdkConfig) -> aws_sdk_ssm::Client {
    let mut ssm_config_builder = aws_sdk_ssm::config::Builder::from(conf);
    if is_test_env() {
        log::info!("Using mocked AWS endpoint for ssm {}", aws_test_endpoint());
        ssm_config_builder = ssm_config_builder.endpoint_url(aws_test_endpoint())
    }
    aws_sdk_ssm::Client::from_conf(ssm_config_builder.build())
}

/// get the data from the ssm parameter store by name
/// Will cache the result for 60s
#[cached(time = 60, result = true)]
pub async fn get_ssm_parameter(name: String) -> Result<String> {
    let store = AwsSsmStore::new().await?;
    store.get_parameter(&name).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashMap};

    struct FakeSsmStore {
        values: HashMap<String, String>,
        failures: BTreeSet<String>,
    }

    #[async_trait]
    impl SsmStore for FakeSsmStore {
        async fn get_parameter(&self, name: &str) -> Result<String> {
            if self.failures.contains(name) {
                return Err(anyhow!("missing parameter: {name}"));
            }

            self.values
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow!("unexpected parameter: {name}"))
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
    async fn test_ssm_get_value_maps_plain_and_json_data() {
        let backend = Backend {
            backend: crd::BackendType::SSM,
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
                    remote_path: Some("db.user".to_string()),
                    key: Some("username".to_string()),
                    configuration_profile_id: None,
                    version_number: None,
                },
            ],
            pulumi_token: None,
        };

        let store = FakeSsmStore {
            values: HashMap::from([
                ("plain-secret".to_string(), "plain-value".to_string()),
                (
                    "json-secret".to_string(),
                    r#"{"db":{"user":"alice"}}"#.to_string(),
                ),
            ]),
            failures: BTreeSet::new(),
        };

        let result = SSM::from_backend(&backend)
            .get_value_with_store(&store)
            .await;

        assert_eq!(
            result.get("token"),
            Some(&ByteString(b"plain-value".to_vec()))
        );
        assert_eq!(result.get("username"), Some(&ByteString(b"alice".to_vec())));
    }

    #[tokio::test]
    async fn test_ssm_get_value_skips_failed_parameters() {
        let backend = Backend {
            backend: crd::BackendType::SSM,
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

        let store = FakeSsmStore {
            values: HashMap::from([("present-secret".to_string(), "ok".to_string())]),
            failures: BTreeSet::from(["missing-secret".to_string()]),
        };

        let result = SSM::from_backend(&backend)
            .get_value_with_store(&store)
            .await;

        assert_eq!(result.len(), 1);
        assert_eq!(result.get("present"), Some(&ByteString(b"ok".to_vec())));
    }

    #[tokio::test]
    async fn test_get_ssm_parameter_mock_smoke() {
        if skip_without_mock_env() {
            return;
        }

        let result = get_ssm_parameter("MyStringParameter".to_string())
            .await
            .unwrap();
        assert_eq!(result, "Vici");
    }
}
