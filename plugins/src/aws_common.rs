use anyhow::Result;

/// if using the local AWS mock backend
pub fn is_test_env() -> bool {
    std::env::var("TEST_ENV").unwrap_or_default() == "true"
}

/// AWS integration tests require the local mock environment described in the repo docs.
pub fn should_run_aws_integration_tests() -> bool {
    is_test_env()
}

/// Get the mock AWS endpoint used in integration tests.
pub fn aws_endpoint_url() -> &'static str {
    let default_endpoint = "http://localhost:8080/".to_string();
    let url = std::env::var("AWS_ENDPOINT_URL").unwrap_or(default_endpoint);
    Box::leak(url.into_boxed_str())
}

pub async fn get_aws_sdk_config() -> Result<aws_types::SdkConfig> {
    Ok(aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await)
}
