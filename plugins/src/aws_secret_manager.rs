use crate::aws_common::{get_aws_sdk_config, is_test_env, localstack_endpoint};
use async_trait::async_trait;
use cached::proc_macro::cached;
use crd::{Backend, RemoteValue, SecretData};

use anyhow::Result;
use k8s_openapi::ByteString;
use std::collections::BTreeMap;

use utils::value::get_secret_data;

pub struct SecretManager {
    data: Vec<SecretData>,
}

#[async_trait]
impl RemoteValue for SecretManager {
    fn from_backend(backend: &Backend) -> SecretManager {
        SecretManager {
            data: backend.data.clone(),
        }
    }

    async fn get_value(&self) -> BTreeMap<String, ByteString> {
        let mut secrets = BTreeMap::new();

        for secret_data in self.data.iter() {
            let data = get_secretsmanager_parameter(secret_data.value.clone()).await;

            match data {
                Ok(data) => {
                    let data = get_secret_data(secret_data, &data);

                    secrets = data
                        .into_iter()
                        .chain(secrets.clone().into_iter())
                        .collect();
                }
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }

        secrets
    }
}

/// get the secret manager client
pub fn secretsmanager_client(conf: &aws_types::SdkConfig) -> aws_sdk_secretsmanager::Client {
    let mut secretsmanager_config_builder = aws_sdk_secretsmanager::config::Builder::from(conf);
    if is_test_env() {
        log::info!(
            "Using localstack for SecretsManager {}",
            localstack_endpoint()
        );
        secretsmanager_config_builder =
            secretsmanager_config_builder.endpoint_url(localstack_endpoint());
    }
    aws_sdk_secretsmanager::Client::from_conf(secretsmanager_config_builder.build())
}

/// get the data from the secret manager store by name
/// Will cache the result for 60s
#[cached(time = 60, result = true)]
pub async fn get_secretsmanager_parameter(name: String) -> Result<String> {
    let shared_config = get_aws_sdk_config().await?;
    let client = secretsmanager_client(&shared_config);
    let output = client.get_secret_value().secret_id(name).send().await?;
    let result = output.secret_string().unwrap_or_default();
    Ok(result.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[tokio::test]
    async fn test_get_secretsmanager_parameter() {
        let result = get_secretsmanager_parameter("MyTestSecret".to_string())
            .await
            .unwrap();
        assert_eq!(result, "Vicd");
    }

    #[tokio::test]
    async fn test_secret_manager() {
        let backend_str = r#"
        {
            "backend": "SecretManager",
            "data": [
                {
                    "value": "MyTestSecret",
                    "key": "value2"
                }
            ]
        }"#;

        let backend: Backend = serde_json::from_str(backend_str).unwrap();

        let client = SecretManager::from_backend(&backend);

        let result = client.get_value();

        let _value = result.await;

        println!("{:?}", _value);

        assert!(true);
    }
}
