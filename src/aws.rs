use std::{collections::BTreeMap, str::FromStr};

use aws_smithy_http::endpoint::Endpoint;
use cached::proc_macro::cached;
use http::Uri;
use k8s_openapi::ByteString;

use crate::Backend;

/// if using localstack as aws backend
pub fn use_localstack() -> bool {
    std::env::var("TEST_ENV").unwrap_or_default() == "true"
}

/// get the localstack endpoint
pub fn localstack_endpoint() -> Endpoint {
    let url = std::env::var("LOCALSTACK_URL").unwrap_or("http://localhost:4566/".to_string());
    Endpoint::immutable(Uri::from_str(&url).unwrap())
}

/// get the ssm client
pub fn ssm_client(conf: &aws_types::SdkConfig) -> aws_sdk_ssm::Client {
    let mut ssm_config_builder = aws_sdk_ssm::config::Builder::from(conf);
    if use_localstack() {
        log::info!("Using localstack for ssm");
        ssm_config_builder = ssm_config_builder.endpoint_resolver(localstack_endpoint())
    }
    aws_sdk_ssm::Client::from_conf(ssm_config_builder.build())
}

/// get the secret manager client
pub fn secretsmanager_client(conf: &aws_types::SdkConfig) -> aws_sdk_secretsmanager::Client {
    let mut secretsmanager_config_builder = aws_sdk_secretsmanager::config::Builder::from(conf);
    if use_localstack() {
        log::info!("Using localstack for SecretsManager");
        secretsmanager_config_builder =
            secretsmanager_config_builder.endpoint_resolver(localstack_endpoint())
    }
    aws_sdk_secretsmanager::Client::from_conf(secretsmanager_config_builder.build())
}

/// get the cloudformation client
pub fn cloudformation_client(conf: &aws_types::SdkConfig) -> aws_sdk_cloudformation::Client {
    let mut cloudformation_config_builder = aws_sdk_cloudformation::config::Builder::from(conf);
    if use_localstack() {
        log::info!("Using localstack for CloudFormation");
        cloudformation_config_builder =
            cloudformation_config_builder.endpoint_resolver(localstack_endpoint())
    }
    aws_sdk_cloudformation::Client::from_conf(cloudformation_config_builder.build())
}

/// get the data from the ssm parameter store by name
/// Will cache the result for 60s
#[cached(time = 60, result = true)]
pub async fn get_ssm_parameter(name: String) -> Result<String, aws_sdk_ssm::Error> {
    let shared_config = aws_config::from_env().load().await;
    let client = ssm_client(&shared_config);
    let parmeter = client.get_parameter().name(name).send().await?;
    let result = parmeter.parameter().unwrap().value().unwrap_or_default();
    Ok(result.to_string())
}

/// get the data from the secret manager store by name
/// Will cache the result for 60s
#[cached(time = 60, result = true)]
pub async fn get_secretsmanager_parameter(
    name: String,
) -> Result<String, aws_sdk_secretsmanager::Error> {
    let shared_config = aws_config::from_env().load().await;
    let client = secretsmanager_client(&shared_config);
    let output = client.get_secret_value().secret_id(name).send().await?;
    let result = output.secret_string().unwrap_or_default();
    Ok(result.to_string())
}

/// get the data from the secret manager store by name
/// Will cache the result for 60s
#[cached(time = 60, result = true)]
pub async fn get_cloudformation_outputs(
    stack_name: String,
) -> Result<Vec<aws_sdk_cloudformation::model::Output>, aws_sdk_cloudformation::Error> {
    let shared_config = aws_config::from_env().load().await;
    let client = cloudformation_client(&shared_config);
    let resp = client
        .describe_stacks()
        .stack_name(stack_name)
        .send()
        .await?;

    let result = resp
        .stacks()
        .unwrap_or_default()
        .first()
        .unwrap()
        .outputs()
        .unwrap_or_default();

    Ok(result.to_owned())
}

/// get the output value from the cloudformation stack
pub async fn get_cloudformation_output(
    stack_name: String,
    output_key: String,
) -> Result<String, aws_sdk_cloudformation::Error> {
    let outputs = get_cloudformation_outputs(stack_name).await?;
    let result = outputs
        .iter()
        .find(|output| output.output_key().unwrap_or_default() == output_key)
        .unwrap()
        .output_value()
        .unwrap_or_default();

    Ok(result.to_string())
}

/// convert the plain text backend data to k8s secret data
pub fn get_plain_text_secret_data(backend: &Backend) -> BTreeMap<String, ByteString> {
    let mut secrets = BTreeMap::new();
    backend.data.iter().for_each(|secret_data| {
        if secret_data.secret_field_name.is_some() {
            secrets.insert(
                secret_data.secret_field_name.clone().unwrap(),
                ByteString(secret_data.name_or_value.clone().as_bytes().to_vec()),
            );
        }
    });

    secrets
}

/// convert the secret manager data to k8s secret data
pub async fn get_secret_manager_secret_data(
    backend: &Backend,
) -> Result<BTreeMap<String, ByteString>, aws_sdk_secretsmanager::Error> {
    let mut secrets = BTreeMap::new();

    for secret_data in backend.data.iter() {
        if secret_data.secret_field_name.is_some() {
            let ssm_secret_data =
                get_secretsmanager_parameter(secret_data.name_or_value.clone()).await;

            let key = secret_data.secret_field_name.clone().unwrap();

            match ssm_secret_data {
                Ok(ssm_secret_data) => {
                    secrets.insert(key, ByteString(ssm_secret_data.clone().into_bytes()));
                }
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }
    }

    Ok(secrets)
}

/// convert the ssm backend data to k8s secret data
pub async fn get_ssm_secret_data(
    backend: &Backend,
) -> Result<BTreeMap<String, ByteString>, aws_sdk_secretsmanager::Error> {
    let mut secrets = BTreeMap::new();

    for secret_data in backend.data.iter() {
        if secret_data.secret_field_name.is_some() {
            let ssm_secret_data = get_ssm_parameter(secret_data.name_or_value.clone()).await;

            let key = secret_data.secret_field_name.clone().unwrap();

            match ssm_secret_data {
                Ok(ssm_secret_data) => {
                    secrets.insert(key, ByteString(ssm_secret_data.clone().into_bytes()));
                }
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }
    }

    Ok(secrets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_ssm_parameter() {
        let result = get_ssm_parameter("MyStringParameter".to_string())
            .await
            .unwrap();
        assert_eq!(result, "Vici");
    }

    #[tokio::test]
    async fn test_get_secretsmanager_parameter() {
        let result = get_secretsmanager_parameter("MyTestSecret".to_string())
            .await
            .unwrap();
        assert_eq!(result, "Vicd");
    }

    #[tokio::test]
    async fn test_get_cloudformation_stack() {
        let result = get_cloudformation_output("MyTestStack".to_string(), "S3Bucket".to_string())
            .await
            .unwrap();

        assert_eq!(result, "S3Bucket");
    }
}
