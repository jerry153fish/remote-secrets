use anyhow::Result;

/// if using localstack as aws backend
pub fn is_test_env() -> bool {
    std::env::var("TEST_ENV").unwrap_or_default() == "true"
}

/// get the localstack endpoint
pub fn localstack_endpoint() -> &'static str {
    let local_endpoint = "http://localhost:4566/".to_string();
    let url = std::env::var("LOCALSTACK_URL").unwrap_or(local_endpoint);
    Box::leak(url.into_boxed_str())
}

pub async fn get_aws_sdk_config() -> Result<aws_types::SdkConfig> {
    Ok(aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await)
}
