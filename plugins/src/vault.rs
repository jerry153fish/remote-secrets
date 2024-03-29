use crate::aws_common::is_test_env;
use async_trait::async_trait;
use std::collections::BTreeMap;

use cached::proc_macro::cached;
use k8s_openapi::ByteString;

use crd::{Backend, RemoteValue, SecretData};
use json_dotpath::DotPaths;

use utils::value::get_secret_data;

use anyhow::Result;

#[derive(Clone, Debug)]
pub struct Vault {
    data: Vec<SecretData>,
}

#[async_trait]
impl RemoteValue for Vault {
    fn from_backend(backend: &Backend) -> Vault {
        Vault {
            data: backend.data.clone(),
        }
    }

    async fn get_value(&self) -> BTreeMap<String, ByteString> {
        let mut secrets = BTreeMap::new();

        for secret_data in self.data.iter() {
            let vault_data = get_vault_value(secret_data.value.clone()).await;
            match vault_data {
                Ok(vault_data) => {
                    let data = get_secret_data(secret_data, &vault_data);

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

        secrets
    }
}

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
    let local_vault_endpoint = "http://localhost:8200".to_string();
    let url = if is_test_env() {
        std::env::var("VAULT_ADDR").unwrap_or(local_vault_endpoint)
    } else {
        std::env::var("VAULT_ADDR")?
    };

    if url.ends_with('/') {
        Ok(format!("{}v1/secret/data/", url))
    } else {
        Ok(format!("{}/v1/secret/data/", url))
    }
}

pub fn get_vault_token() -> Result<String> {
    let default_vault_root_token = "vault-plaintext-root-token".to_string();
    let token = if is_test_env() {
        std::env::var("VAULT_TOKEN").unwrap_or(default_vault_root_token)
    } else {
        std::env::var("VAULT_TOKEN")?
    };
    Ok(token)
}

pub fn get_vault_client(key: String) -> Result<reqwest::RequestBuilder> {
    let endpoint = get_vault_secret_endpoint()?;
    let token = get_vault_token()?;

    let url = format!("{}{}", endpoint, key);

    let client = reqwest::Client::new()
        .get(url)
        .header("X-Vault-Token", token);

    Ok(client)
}

mod tests {
    #![allow(unused_imports)]
    use super::*;

    #[tokio::test]
    async fn test_get_vault_string_value() {
        let rest = get_vault_value("vaultString".to_string()).await.unwrap();

        assert_eq!(rest, "\"vaultString\"".to_string());
    }

    #[tokio::test]
    async fn test_get_vault_json_value() {
        let rest = get_vault_value("vaultJson".to_string()).await.unwrap();

        assert_eq!(
            rest,
            "{\"vaultJson1\":\"vaultJson1\",\"vaultJson2\":\"vaultJson2\"}".to_string()
        );
    }
}
