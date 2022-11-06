use crate::aws_common::{is_test_env, localstack_endpoint};
use async_trait::async_trait;
use cached::proc_macro::cached;
use crd::{Backend, RemoteValue, SecretData};

use anyhow::{anyhow, Result};
use k8s_openapi::ByteString;
use std::collections::BTreeMap;

use utils::value::get_secret_data;

pub struct Cloudformation {
    data: Vec<SecretData>,
}

#[async_trait]
impl RemoteValue for Cloudformation {
    fn from_backend(backend: &Backend) -> Cloudformation {
        Cloudformation {
            data: backend.data.clone(),
        }
    }

    async fn get_value(&self) -> BTreeMap<String, ByteString> {
        let mut secrets = BTreeMap::new();

        for secret_data in self.data.iter() {
            // specific the output value for 1-1 mapping k8s secret key
            // TODO: support the output value is not dict
            if secret_data.key.is_some() && secret_data.remote_path.is_some() {
                let cloudformation_secret_data = get_cloudformation_output(
                    secret_data.value.clone(),
                    secret_data.remote_path.clone().unwrap(),
                )
                .await;

                match cloudformation_secret_data {
                    Ok(cloudformation_secret_data) => {
                        let data = get_secret_data(secret_data, &cloudformation_secret_data);

                        secrets = data
                            .into_iter()
                            .chain(secrets.clone().into_iter())
                            .collect();
                    }
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            } else {
                // insert the whole cloudformation outputs into k8s secret data
                let cloudformation_secret_data =
                    get_cloudformation_outputs_as_secret_data(secret_data.value.clone()).await;

                match cloudformation_secret_data {
                    Ok(cloudformation_secret_data) => {
                        secrets = cloudformation_secret_data
                            .into_iter()
                            .chain(secrets.clone().into_iter())
                            .collect();
                    }
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            }
        }

        secrets
    }
}

/// get the cloudformation client
pub fn cloudformation_client(conf: &aws_types::SdkConfig) -> aws_sdk_cloudformation::Client {
    let mut cloudformation_config_builder = aws_sdk_cloudformation::config::Builder::from(conf);
    if is_test_env() {
        log::info!("Using localstack for CloudFormation");
        cloudformation_config_builder =
            cloudformation_config_builder.endpoint_resolver(localstack_endpoint())
    }
    aws_sdk_cloudformation::Client::from_conf(cloudformation_config_builder.build())
}

/// get the data from the secret manager store by name
/// Will cache the result for 60s
#[cached(time = 60, result = true)]
pub async fn get_cloudformation_outputs(
    stack_name: String,
) -> Result<Vec<aws_sdk_cloudformation::model::Output>> {
    let shared_config = aws_config::from_env().load().await;
    let client = cloudformation_client(&shared_config);
    let resp = client
        .describe_stacks()
        .stack_name(stack_name)
        .send()
        .await?;

    let result = resp
        .stacks()
        .ok_or_else(|| anyhow!("no stacks found"))?
        .first()
        .ok_or_else(|| anyhow!("no first stack found"))?
        .outputs()
        .unwrap_or_default();

    Ok(result.to_owned())
}

/// get the output value from the cloudformation stack
pub async fn get_cloudformation_output(stack_name: String, remote_path: String) -> Result<String> {
    let outputs = get_cloudformation_outputs(stack_name).await?;
    let result = outputs
        .iter()
        .find(|output| output.output_key().unwrap_or_default() == remote_path)
        .ok_or_else(|| anyhow!("no output found"))?
        .output_value()
        .unwrap_or_default();

    Ok(result.to_string())
}

// get the secret data from the whole outputs of the cloudformation stack
pub async fn get_cloudformation_outputs_as_secret_data(
    stack_name: String,
) -> Result<BTreeMap<String, ByteString>> {
    let outputs = get_cloudformation_outputs(stack_name).await?;
    let mut secrets = BTreeMap::new();
    for output in outputs {
        let remote_path = output.output_key().unwrap_or_default().to_owned();
        let output_value = ByteString(
            output
                .output_value()
                .unwrap_or_default()
                .as_bytes()
                .to_vec(),
        );
        secrets.insert(remote_path, output_value);
    }

    Ok(secrets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[tokio::test]
    async fn test_get_cloudformation_stack() {
        let result = get_cloudformation_output("MyTestStack".to_string(), "S3Bucket".to_string())
            .await
            .unwrap();

        assert_eq!(result, "S3Bucket");
    }

    #[tokio::test]
    async fn test_get_cloudformation_outputs() {
        let result = get_cloudformation_outputs_as_secret_data("MyTestStack".to_string())
            .await
            .unwrap();

        let data_string = serde_json::to_string(&result).unwrap();

        assert_eq!(data_string.contains("UzNCdWNrZXQ"), true);
    }

    #[tokio::test]
    async fn test_cloudformation() {
        let backend_str = r#"
        {
            "backend": "Cloudformation",
            "data": [
                {
                    "value": "MyTestStack",
                    "key": "value1"
                }
            ]
        }"#;

        let backend: Backend = serde_json::from_str(backend_str).unwrap();

        let cfn = Cloudformation::from_backend(&backend);

        let result = cfn.get_value();

        let _value = result.await;

        println!("{:?}", _value);

        assert!(true);
    }
}
