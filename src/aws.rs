use std::{collections::BTreeMap, str::FromStr};

use aws_smithy_http::endpoint::Endpoint;
use cached::proc_macro::cached;
use futures::future::ok;
use http::Uri;
use k8s_openapi::ByteString;

use crate::{utils, Backend};

/// if using localstack as aws backend
pub fn is_test_env() -> bool {
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
    if is_test_env() {
        log::info!("Using localstack for ssm");
        ssm_config_builder = ssm_config_builder.endpoint_resolver(localstack_endpoint())
    }
    aws_sdk_ssm::Client::from_conf(ssm_config_builder.build())
}

/// get the secret manager client
pub fn secretsmanager_client(conf: &aws_types::SdkConfig) -> aws_sdk_secretsmanager::Client {
    let mut secretsmanager_config_builder = aws_sdk_secretsmanager::config::Builder::from(conf);
    if is_test_env() {
        log::info!("Using localstack for SecretsManager");
        secretsmanager_config_builder =
            secretsmanager_config_builder.endpoint_resolver(localstack_endpoint())
    }
    aws_sdk_secretsmanager::Client::from_conf(secretsmanager_config_builder.build())
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

/// get the cloudformation client
pub fn appconfig_client(conf: &aws_types::SdkConfig) -> aws_sdk_appconfig::Client {
    let appconfig_config_builder = aws_sdk_appconfig::config::Builder::from(conf);

    aws_sdk_appconfig::Client::from_conf(appconfig_config_builder.build())
}

/// get the data from the ssm parameter store by name
/// Will cache the result for 60s
#[cached(time = 60, result = true)]
pub async fn get_appconfig_configuration_by_version(
    application_id: String,
    configuration_profile_id: String,
    version_number: i32,
) -> Result<String, aws_sdk_appconfig::Error> {
    let shared_config = aws_config::from_env().load().await;
    let client = appconfig_client(&shared_config);
    let result = client
        .get_hosted_configuration_version()
        .application_id(application_id)
        .configuration_profile_id(configuration_profile_id)
        .version_number(version_number)
        .send()
        .await
        .unwrap()
        .content()
        .unwrap()
        .to_owned()
        .into_inner();

    let res = std::string::String::from_utf8(result).unwrap();

    Ok(res)
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

// get the secret data from the whole outputs of the cloudformation stack
pub async fn get_cloudformation_outputs_as_secret_data(
    stack_name: String,
) -> Result<BTreeMap<String, ByteString>, aws_sdk_cloudformation::Error> {
    let outputs = get_cloudformation_outputs(stack_name).await?;
    let mut secrets = BTreeMap::new();
    for output in outputs {
        let output_key = output.output_key().unwrap_or_default().to_owned();
        let output_value = ByteString(
            output
                .output_value()
                .unwrap_or_default()
                .as_bytes()
                .to_vec(),
        );
        secrets.insert(output_key, output_value);
    }

    Ok(secrets)
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
        let secret_manager_data =
            get_secretsmanager_parameter(secret_data.name_or_value.clone()).await;
        match secret_manager_data {
            Ok(secret_manager_data) => {
                let data =
                    utils::rsecret_data_to_secret_data(secret_data, &secret_manager_data).unwrap();

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

    Ok(secrets)
}

/// convert the secret manager data to k8s secret data
pub async fn get_cloudformation_stack_secret_data(
    backend: &Backend,
) -> Result<BTreeMap<String, ByteString>, aws_sdk_secretsmanager::Error> {
    let mut secrets = BTreeMap::new();

    for secret_data in backend.data.iter() {
        // specific the output value for 1-1 mapping k8s secret key
        // TODO: support the output value is not dict
        if secret_data.secret_field_name.is_some() && secret_data.output_key.is_some() {
            let cloudformation_secret_data = get_cloudformation_output(
                secret_data.name_or_value.clone(),
                secret_data.output_key.clone().unwrap(),
            )
            .await;

            match cloudformation_secret_data {
                Ok(cloudformation_secret_data) => {
                    let data = utils::rsecret_data_to_secret_data(
                        secret_data,
                        &cloudformation_secret_data,
                    )
                    .unwrap();

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
                get_cloudformation_outputs_as_secret_data(secret_data.name_or_value.clone()).await;

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

    Ok(secrets)
}

/// convert the ssm backend data to k8s secret data
pub async fn get_ssm_secret_data(
    backend: &Backend,
) -> Result<BTreeMap<String, ByteString>, aws_sdk_secretsmanager::Error> {
    let mut secrets = BTreeMap::new();

    for secret_data in backend.data.iter() {
        let ssm_secret_data = get_ssm_parameter(secret_data.name_or_value.clone()).await;

        match ssm_secret_data {
            Ok(ssm_secret_data) => {
                let data =
                    utils::rsecret_data_to_secret_data(secret_data, &ssm_secret_data).unwrap();

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

    #[tokio::test]
    async fn test_get_cloudformation_outputs() {
        let result = get_cloudformation_outputs_as_secret_data("MyTestStack".to_string())
            .await
            .unwrap();

        let data_string = serde_json::to_string(&result).unwrap();

        assert_eq!(data_string.contains("UzNCdWNrZXQ"), true);
    }
}
