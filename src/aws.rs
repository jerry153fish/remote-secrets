use aws_sdk_ssm::{Client, Error};
use aws_smithy_http::endpoint::Endpoint;
use http::Uri;

pub fn use_localstack() -> bool {
    std::env::var("TEST_ENV").unwrap_or_default() == "true"
}

pub fn localstack_endpoint() -> Endpoint {
    Endpoint::immutable(Uri::from_static("http://localhost:4566/"))
}

pub fn ssm_client(conf: &aws_types::SdkConfig) -> aws_sdk_ssm::Client {
    let mut ssm_config_builder = aws_sdk_ssm::config::Builder::from(conf);
    if use_localstack() {
        ssm_config_builder = ssm_config_builder.endpoint_resolver(localstack_endpoint())
    }
    Client::from_conf(ssm_config_builder.build())
}

pub async fn get_ssm_parameter(name: &str) -> Result<String, Error> {
    let shared_config = aws_config::from_env().load().await;
    let client = ssm_client(&shared_config);
    let test = client.get_parameter().name(name).send().await.unwrap();
    let result = test.parameter().unwrap().value().unwrap_or_default();
    Ok(result.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_ssm_parameter() {
        let result = get_ssm_parameter("MyStringParameter").await.unwrap();
        assert_eq!(result, "Vici");
    }
}
