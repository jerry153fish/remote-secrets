use std::str::FromStr;

use aws_smithy_http::endpoint::Endpoint;
use cached::proc_macro::cached;
use http::Uri;

pub fn use_localstack() -> bool {
    std::env::var("TEST_ENV").unwrap_or_default() == "true"
}

pub fn localstack_endpoint() -> Endpoint {
    let url = std::env::var("LOCALSTACK_URL").unwrap_or("http://localhost:4566/".to_string());
    Endpoint::immutable(Uri::from_str(&url).unwrap())
}

pub fn ssm_client(conf: &aws_types::SdkConfig) -> aws_sdk_ssm::Client {
    let mut ssm_config_builder = aws_sdk_ssm::config::Builder::from(conf);
    if use_localstack() {
        log::info!("Using localstack for ssm");
        ssm_config_builder = ssm_config_builder.endpoint_resolver(localstack_endpoint())
    }
    aws_sdk_ssm::Client::from_conf(ssm_config_builder.build())
}

pub fn secretsmanager_client(conf: &aws_types::SdkConfig) -> aws_sdk_secretsmanager::Client {
    let mut secretsmanager_config_builder = aws_sdk_secretsmanager::config::Builder::from(conf);
    if use_localstack() {
        log::info!("Using localstack for SecretsManager");
        secretsmanager_config_builder =
            secretsmanager_config_builder.endpoint_resolver(localstack_endpoint())
    }
    aws_sdk_secretsmanager::Client::from_conf(secretsmanager_config_builder.build())
}

pub fn cloudformation_client(conf: &aws_types::SdkConfig) -> aws_sdk_cloudformation::Client {
    let mut cloudformation_config_builder = aws_sdk_cloudformation::config::Builder::from(conf);
    if use_localstack() {
        log::info!("Using localstack for CloudFormation");
        cloudformation_config_builder =
            cloudformation_config_builder.endpoint_resolver(localstack_endpoint())
    }
    aws_sdk_cloudformation::Client::from_conf(cloudformation_config_builder.build())
}

#[cached(time = 60, result = true)]
pub async fn get_ssm_parameter(name: String) -> Result<String, aws_sdk_ssm::Error> {
    let shared_config = aws_config::from_env().load().await;
    let client = ssm_client(&shared_config);
    let parmeter = client.get_parameter().name(name).send().await?;
    let result = parmeter.parameter().unwrap().value().unwrap_or_default();
    Ok(result.to_string())
}

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

#[cached(time = 60, result = true)]
pub async fn get_cloudformation_output(
    stack_name: String,
    output_key: String,
) -> Result<String, aws_sdk_cloudformation::Error> {
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
        .unwrap_or_default()
        .into_iter()
        .find(|&o| o.output_key() == Some(&output_key))
        .unwrap()
        .output_value()
        .unwrap_or_default();

    Ok(result.to_string())
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
