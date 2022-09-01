use std::str::FromStr;

use aws_smithy_http::endpoint::Endpoint;
use cached::proc_macro::cached;
use http::Uri;

use anyhow::{anyhow, Result};

/// if using localstack as aws backend
pub fn is_test_env() -> bool {
    std::env::var("TEST_ENV").unwrap_or_default() == "true"
}

/// get the localstack endpoint
pub fn localstack_endpoint() -> Endpoint {
    let url = std::env::var("LOCALSTACK_URL").unwrap_or("http://localhost:4566/".to_string());
    Endpoint::immutable(Uri::from_str(&url).unwrap())
}

/// get the cloudformation client
pub fn appconfig_client(conf: &aws_types::SdkConfig) -> aws_sdk_appconfig::Client {
    let appconfig_config_builder = aws_sdk_appconfig::config::Builder::from(conf);

    aws_sdk_appconfig::Client::from_conf(appconfig_config_builder.build())
}

// pub async fn catch_sdk_config_panic() -> Result<aws_types::SdkConfig> {
//     let my_complex_type = 1;

//     // Wrap it all in a std::sync::Mutex
//     let mutex = std::sync::Mutex::new(my_complex_type);
//     let result = panic::catch_unwind(|| {
//         let my_complex_type = mutex.lock().unwrap();

//         // Enter the runtime
//         let handle = tokio::runtime::Handle::current();

//         handle.enter();

//         futures::executor::block_on(get_aws_sdk_config(*my_complex_type))
//     });

//     match result {
//         Ok(result) => match result {
//             Ok(result) => Ok(result),
//             Err(err) => {
//                 log::error!("{:?}", err);
//                 Err(anyhow!("Failed to get the credentials"))
//             }
//         },
//         Err(err) => {
//             log::error!("{:?}", err);
//             Err(anyhow!("Failed to get the credentials"))
//         }
//     }
// }

pub async fn get_aws_sdk_config() -> Result<aws_types::SdkConfig> {
    Ok(aws_config::from_env().load().await)
}
/// get the data from the ssm parameter store by name
/// Will cache the result for 60s
#[cached(time = 60, result = true)]
pub async fn get_appconfig_configuration_by_version(
    application_id: String,
    configuration_profile_id: String,
    version_number: i32,
) -> Result<String> {
    let shared_config = get_aws_sdk_config().await?;
    let client = appconfig_client(&shared_config);
    let result = client
        .get_hosted_configuration_version()
        .application_id(application_id)
        .configuration_profile_id(configuration_profile_id)
        .version_number(version_number)
        .send()
        .await?
        .content()
        .ok_or(anyhow!("no content"))?
        .to_owned()
        .into_inner();

    let res = std::string::String::from_utf8(result)?;

    Ok(res)
}
