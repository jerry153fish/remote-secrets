use std::{collections::BTreeMap, env::VarError};

use cached::proc_macro::cached;
use k8s_openapi::ByteString;

use std::str::FromStr;

use aws_smithy_http::endpoint::Endpoint;
use futures::future::ok;
use http::Uri;

use crate::{utils, Backend};

use crate::aws;
use json_dotpath::DotPaths;

use anyhow::Result;

#[cached(time = 60, result = true)]
pub async fn get_vault_value(path: String) -> Result<String> {
    let client = get_vault_client(path.clone())?;
    let response: serde_json::Value = client.send().await?.json().await?;

    let result = response
        .dot_get::<serde_json::Value>("data.data.value")
        .unwrap()
        .unwrap_or_default()
        .to_string();

    Ok(result)
}

pub fn get_vault_secret_endpoint() -> Result<String> {
    let url;
    if aws::is_test_env() {
        url = std::env::var("VAULT_ADDR").unwrap_or("http://localhost:8200".to_string());
    } else {
        url = std::env::var("VAULT_ADDR")?;
    }
    if url.ends_with("/") {
        Ok(format!("{}v1/secret/data/", url))
    } else {
        Ok(format!("{}/v1/secret/data/", url))
    }
}

pub fn get_vault_token() -> Result<String> {
    let token;
    if aws::is_test_env() {
        token = std::env::var("VAULT_TOKEN").unwrap_or("vault-plaintext-root-token".to_string());
    } else {
        token = std::env::var("VAULT_TOKEN")?;
    }
    Ok(token)
}

pub fn get_vault_client(key: String) -> Result<reqwest::RequestBuilder> {
    let endpoint = get_vault_secret_endpoint()?;
    let token = get_vault_token()?;

    let url = format!("{}{}", endpoint, key.clone());

    let client = reqwest::Client::new()
        .get(url)
        .header("X-Vault-Token", token);

    Ok(client)
}

/// convert the vault data to k8s secret data
pub async fn get_vault_secret_data(backend: &Backend) -> Result<BTreeMap<String, ByteString>> {
    let mut secrets = BTreeMap::new();

    for secret_data in backend.data.iter() {
        let vault_data = get_vault_value(secret_data.remote_value.clone()).await;
        match vault_data {
            Ok(vault_data) => {
                let data = utils::rsecret_data_to_secret_data(secret_data, &vault_data);

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
/// convert the vault backend data to k8s secret data
// pub fn get_vault_secret_data(
//     backend: &Backend,
// ) -> Result<BTreeMap<String, ByteString>, hashicorp_vault::Error> {
//     let mut secrets = BTreeMap::new();

//     for secret_data in backend.data.iter() {
//         let vault_data = get_vault_value(secret_data.remote_value.as_str());
//         match vault_data {
//             Ok(vault_data) => {
//                 let data = utils::rsecret_data_to_secret_data(secret_data, &vault_data).unwrap();

//                 secrets = data
//                     .into_iter()
//                     .chain(secrets.clone().into_iter())
//                     .collect();
//             }
//             Err(err) => {
//                 log::error!("{}", err);
//             }
//         }
//     }

//     Ok(secrets)
// }

mod tests {
    use super::*;
    use json_dotpath::DotPaths;

    #[tokio::test]
    async fn test_get_vault_value() {
        let result2 = get_vault_value("baz".to_string()).await.unwrap();

        assert_eq!(result2, "\"bar\"".to_string());
    }
}
