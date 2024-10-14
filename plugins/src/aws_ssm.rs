use crate::aws_common::{get_aws_sdk_config, is_test_env, localstack_endpoint};
use async_trait::async_trait;
use cached::proc_macro::cached;
use crd::{Backend, RemoteValue, SecretData};

use anyhow::{anyhow, Result};
use k8s_openapi::ByteString;
use std::collections::BTreeMap;

use utils::value::get_secret_data;

pub struct SSM {
    data: Vec<SecretData>,
}

#[async_trait]
impl RemoteValue for SSM {
    fn from_backend(backend: &Backend) -> SSM {
        SSM {
            data: backend.data.clone(),
        }
    }

    async fn get_value(&self) -> BTreeMap<String, ByteString> {
        let mut secrets = BTreeMap::new();

        for secret_data in self.data.iter() {
            let ssm_secret_data = get_ssm_parameter(secret_data.value.clone()).await;

            match ssm_secret_data {
                Ok(ssm_secret_data) => {
                    let data = get_secret_data(secret_data, &ssm_secret_data);

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

/// get the ssm client
pub fn ssm_client(conf: &aws_types::SdkConfig) -> aws_sdk_ssm::Client {
    let mut ssm_config_builder = aws_sdk_ssm::config::Builder::from(conf);
    if is_test_env() {
        log::info!("Using localstack for ssm {}", localstack_endpoint());
        ssm_config_builder = ssm_config_builder.endpoint_url(localstack_endpoint())
    }
    aws_sdk_ssm::Client::from_conf(ssm_config_builder.build())
}

/// get the data from the ssm parameter store by name
/// Will cache the result for 60s
#[cached(time = 60, result = true)]
pub async fn get_ssm_parameter(name: String) -> Result<String> {
    let shared_config = get_aws_sdk_config().await?;
    let client = ssm_client(&shared_config);
    let parmeter = client.get_parameter().name(name).send().await?;
    let result = parmeter
        .parameter()
        .ok_or_else(|| anyhow!("no parameter found"))?
        .value()
        .unwrap_or_default();
    Ok(result.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[tokio::test]
    async fn test_get_ssm_parameter() {
        let result = get_ssm_parameter("MyStringParameter".to_string())
            .await
            .unwrap();
        assert_eq!(result, "Vici");
    }

    #[tokio::test]
    async fn test_ssm() {
        let backend_str = r#"
        {
            "backend": "SSM",
            "data": [
                {
                    "value": "MyStringParameter",
                    "key": "value1"
                }
            ]
        }"#;

        let backend: Backend = serde_json::from_str(backend_str).unwrap();

        let ssm = SSM::from_backend(&backend);

        let result = ssm.get_value();

        let _value = result.await;

        println!("{:?}", _value);

        assert!(true);
    }
}
