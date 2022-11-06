use anyhow::Result;
use aws_smithy_http::endpoint::Endpoint;
use http::Uri;
use std::str::FromStr;

/// if using localstack as aws backend
pub fn is_test_env() -> bool {
    std::env::var("TEST_ENV").unwrap_or_default() == "true"
}

/// get the localstack endpoint
pub fn localstack_endpoint() -> Endpoint {
    let local_endpoint = "http://localhost:4566/".to_string();
    let url = std::env::var("LOCALSTACK_URL").unwrap_or(local_endpoint);
    Endpoint::immutable(Uri::from_str(&url).unwrap())
}

pub async fn get_aws_sdk_config() -> Result<aws_types::SdkConfig> {
    Ok(aws_config::from_env().load().await)
}
