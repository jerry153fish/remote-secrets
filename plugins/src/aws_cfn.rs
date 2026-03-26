use crate::aws_common::{aws_test_endpoint, get_aws_sdk_config, is_test_env};
use async_trait::async_trait;
use cached::proc_macro::cached;
use crd::{Backend, RemoteValue, SecretData};

use anyhow::{anyhow, Result};
use k8s_openapi::ByteString;
use std::collections::BTreeMap;
use std::time::Duration;

use utils::value::{get_secret_data, merge_secret_data};

pub struct Cloudformation {
    data: Vec<SecretData>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudformationOutput {
    key: String,
    value: String,
}

#[async_trait]
trait CloudformationStore {
    async fn get_outputs(&self, stack_name: &str) -> Result<Vec<CloudformationOutput>>;
}

struct AwsCloudformationStore {
    client: aws_sdk_cloudformation::Client,
}

impl AwsCloudformationStore {
    async fn new() -> Result<Self> {
        let shared_config = get_aws_sdk_config().await?;
        Ok(Self {
            client: cloudformation_client(&shared_config),
        })
    }
}

#[async_trait]
impl CloudformationStore for AwsCloudformationStore {
    async fn get_outputs(&self, stack_name: &str) -> Result<Vec<CloudformationOutput>> {
        let resp = self
            .client
            .describe_stacks()
            .stack_name(stack_name)
            .send()
            .await?;

        let outputs = resp
            .stacks()
            .first()
            .ok_or_else(|| anyhow!("no first stack found"))?
            .outputs()
            .iter()
            .map(|output| CloudformationOutput {
                key: output.output_key().unwrap_or_default().to_string(),
                value: output.output_value().unwrap_or_default().to_string(),
            })
            .collect();

        Ok(outputs)
    }
}

async fn get_cloudformation_outputs_with_store<S: CloudformationStore + Send + Sync>(
    store: &S,
    stack_name: &str,
) -> Result<Vec<CloudformationOutput>> {
    store.get_outputs(stack_name).await
}

async fn get_cloudformation_output_with_store<S: CloudformationStore + Send + Sync>(
    store: &S,
    stack_name: &str,
    remote_path: &str,
) -> Result<String> {
    let outputs = get_cloudformation_outputs_with_store(store, stack_name).await?;
    let result = outputs
        .iter()
        .find(|output| output.key == remote_path)
        .ok_or_else(|| anyhow!("no output found"))?;

    Ok(result.value.clone())
}

async fn get_cloudformation_outputs_as_secret_data_with_store<
    S: CloudformationStore + Send + Sync,
>(
    store: &S,
    stack_name: &str,
) -> Result<BTreeMap<String, ByteString>> {
    let outputs = get_cloudformation_outputs_with_store(store, stack_name).await?;
    let mut secrets = BTreeMap::new();
    for output in outputs {
        secrets.insert(output.key, ByteString(output.value.into_bytes()));
    }

    Ok(secrets)
}

impl Cloudformation {
    async fn get_value_with_store<S: CloudformationStore + Send + Sync>(
        &self,
        store: &S,
    ) -> BTreeMap<String, ByteString> {
        let mut secrets = BTreeMap::new();

        for secret_data in &self.data {
            if let (Some(_), Some(remote_path)) = (&secret_data.key, &secret_data.remote_path) {
                match get_cloudformation_output_with_store(store, &secret_data.value, remote_path)
                    .await
                {
                    Ok(value) => {
                        let data = get_secret_data(secret_data, &value);
                        secrets = merge_secret_data(data, secrets);
                    }
                    Err(err) => {
                        log::error!("{err}");
                    }
                }
            } else {
                match get_cloudformation_outputs_as_secret_data_with_store(
                    store,
                    &secret_data.value,
                )
                .await
                {
                    Ok(data) => {
                        secrets = merge_secret_data(data, secrets);
                    }
                    Err(err) => {
                        log::error!("{err}");
                    }
                }
            }
        }

        secrets
    }
}

#[async_trait]
impl RemoteValue for Cloudformation {
    fn from_backend(backend: &Backend) -> Cloudformation {
        Cloudformation {
            data: backend.data.clone(),
        }
    }

    async fn get_value(&self) -> BTreeMap<String, ByteString> {
        match AwsCloudformationStore::new().await {
            Ok(store) => self.get_value_with_store(&store).await,
            Err(err) => {
                log::error!("{err}");
                BTreeMap::new()
            }
        }
    }
}

/// get the cloudformation client
pub fn cloudformation_client(conf: &aws_types::SdkConfig) -> aws_sdk_cloudformation::Client {
    let mut cloudformation_config_builder = aws_sdk_cloudformation::config::Builder::from(conf);
    if is_test_env() {
        log::info!(
            "Using mocked AWS endpoint for CloudFormation: {}",
            aws_test_endpoint()
        );
        cloudformation_config_builder =
            cloudformation_config_builder.endpoint_url(aws_test_endpoint())
    }
    aws_sdk_cloudformation::Client::from_conf(cloudformation_config_builder.build())
}

/// get the data from the secret manager store by name
/// Will cache the result for 60s
#[cached(time = 60, result = true)]
pub async fn get_cloudformation_outputs(stack_name: String) -> Result<Vec<CloudformationOutput>> {
    let store = AwsCloudformationStore::new().await?;
    get_cloudformation_outputs_with_store(&store, &stack_name).await
}

/// get the output value from the cloudformation stack
pub async fn get_cloudformation_output(stack_name: String, remote_path: String) -> Result<String> {
    let store = AwsCloudformationStore::new().await?;
    get_cloudformation_output_with_store(&store, &stack_name, &remote_path).await
}

// get the secret data from the whole outputs of the cloudformation stack
pub async fn get_cloudformation_outputs_as_secret_data(
    stack_name: String,
) -> Result<BTreeMap<String, ByteString>> {
    let store = AwsCloudformationStore::new().await?;
    get_cloudformation_outputs_as_secret_data_with_store(&store, &stack_name).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashMap};

    struct FakeCloudformationStore {
        values: HashMap<String, Vec<CloudformationOutput>>,
        failures: BTreeSet<String>,
    }

    #[async_trait]
    impl CloudformationStore for FakeCloudformationStore {
        async fn get_outputs(&self, stack_name: &str) -> Result<Vec<CloudformationOutput>> {
            if self.failures.contains(stack_name) {
                return Err(anyhow!("missing stack: {stack_name}"));
            }

            self.values
                .get(stack_name)
                .cloned()
                .ok_or_else(|| anyhow!("unexpected stack: {stack_name}"))
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
    async fn test_cloudformation_get_value_maps_named_and_bulk_outputs() {
        let backend = Backend {
            backend: crd::BackendType::Cloudformation,
            data: vec![
                SecretData {
                    value: "app-stack".to_string(),
                    is_json_string: None,
                    remote_path: Some("BucketName".to_string()),
                    key: Some("bucket".to_string()),
                    configuration_profile_id: None,
                    version_number: None,
                },
                SecretData {
                    value: "bulk-stack".to_string(),
                    is_json_string: None,
                    remote_path: None,
                    key: None,
                    configuration_profile_id: None,
                    version_number: None,
                },
            ],
            pulumi_token: None,
        };

        let store = FakeCloudformationStore {
            values: HashMap::from([
                (
                    "app-stack".to_string(),
                    vec![CloudformationOutput {
                        key: "BucketName".to_string(),
                        value: "primary-bucket".to_string(),
                    }],
                ),
                (
                    "bulk-stack".to_string(),
                    vec![
                        CloudformationOutput {
                            key: "QueueUrl".to_string(),
                            value: "https://queue".to_string(),
                        },
                        CloudformationOutput {
                            key: "TopicArn".to_string(),
                            value: "arn:aws:sns:::topic".to_string(),
                        },
                    ],
                ),
            ]),
            failures: BTreeSet::new(),
        };

        let result = Cloudformation::from_backend(&backend)
            .get_value_with_store(&store)
            .await;

        assert_eq!(
            result.get("bucket"),
            Some(&ByteString(b"primary-bucket".to_vec()))
        );
        assert_eq!(
            result.get("QueueUrl"),
            Some(&ByteString(b"https://queue".to_vec()))
        );
        assert_eq!(
            result.get("TopicArn"),
            Some(&ByteString(b"arn:aws:sns:::topic".to_vec()))
        );
    }

    #[tokio::test]
    async fn test_cloudformation_get_value_skips_failed_stacks() {
        let backend = Backend {
            backend: crd::BackendType::Cloudformation,
            data: vec![
                SecretData {
                    value: "good-stack".to_string(),
                    is_json_string: None,
                    remote_path: Some("BucketName".to_string()),
                    key: Some("bucket".to_string()),
                    configuration_profile_id: None,
                    version_number: None,
                },
                SecretData {
                    value: "missing-stack".to_string(),
                    is_json_string: None,
                    remote_path: None,
                    key: None,
                    configuration_profile_id: None,
                    version_number: None,
                },
            ],
            pulumi_token: None,
        };

        let store = FakeCloudformationStore {
            values: HashMap::from([(
                "good-stack".to_string(),
                vec![CloudformationOutput {
                    key: "BucketName".to_string(),
                    value: "primary-bucket".to_string(),
                }],
            )]),
            failures: BTreeSet::from(["missing-stack".to_string()]),
        };

        let result = Cloudformation::from_backend(&backend)
            .get_value_with_store(&store)
            .await;

        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("bucket"),
            Some(&ByteString(b"primary-bucket".to_vec()))
        );
    }

    #[tokio::test]
    async fn test_get_cloudformation_output_mock_smoke() {
        if skip_without_mock_env() {
            return;
        }

        let result = get_cloudformation_output("MyTestStack".to_string(), "S3Bucket".to_string())
            .await
            .unwrap();

        assert_eq!(result, "S3Bucket");
    }

    #[tokio::test]
    async fn test_get_cloudformation_outputs_as_secret_data_mock_smoke() {
        if skip_without_mock_env() {
            return;
        }

        let result = get_cloudformation_outputs_as_secret_data("MyTestStack".to_string())
            .await
            .unwrap();

        assert_eq!(
            result.get("S3Bucket"),
            Some(&ByteString(b"S3Bucket".to_vec()))
        );
    }
}
