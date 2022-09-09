use async_trait::async_trait;
use std::collections::BTreeMap;

use cached::proc_macro::cached;
use k8s_openapi::ByteString;

use crd::{Backend, RemoteValue, SecretData};
use json_dotpath::DotPaths;

use anyhow::Result;

use utils::value::get_secret_data;

#[derive(Clone, Debug)]
pub struct Pulumi {
    data: Vec<SecretData>,
    token: Option<String>,
}

#[async_trait]
impl RemoteValue for Pulumi {
    fn from_backend(backend: &Backend) -> Pulumi {
        Pulumi {
            data: backend.data.clone(),
            token: backend.pulumi_token.clone(),
        }
    }

    async fn get_value(&self) -> BTreeMap<String, ByteString> {
        let mut secrets = BTreeMap::new();

        for secret_data in self.data.iter() {
            // specific the output value for 1-1 mapping k8s secret key
            // TODO: support the output value is not dict
            if secret_data.key.is_some() && secret_data.remote_path.is_some() {
                let data = get_pulumi_output(
                    secret_data.value.clone(),
                    secret_data.remote_path.clone().unwrap(),
                    self.token.clone(),
                )
                .await;

                match data {
                    Ok(data) => {
                        let data = get_secret_data(secret_data, &data);

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
                let data = get_pulumi_outputs_as_secret_data(
                    secret_data.value.clone(),
                    self.token.clone(),
                )
                .await;

                match data {
                    Ok(data) => {
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
        }

        secrets
    }
}

#[cached(time = 60, result = true)]
pub async fn get_pulumi_outputs(
    path: String,
    pulumi_token: Option<String>,
) -> Result<serde_json::Value> {
    let client = get_pulumi_client(path.clone(), pulumi_token)?;
    let response: serde_json::Value = client.send().await?.json().await?;

    let result = response
        .dot_get::<serde_json::Value>("deployment.resources.0.outputs")
        .unwrap()
        .unwrap_or_default();

    Ok(result)
}

pub fn get_pulumi_endpoint() -> Result<String> {
    Ok(std::env::var("PULUMI_ENDPOINT").unwrap_or("https://api.pulumi.com/api/stacks".to_string()))
}

pub fn get_pulumi_token(pulumi_token: Option<String>) -> Result<String> {
    let token;
    match pulumi_token {
        None => token = std::env::var("PULUMI_ACCESS_TOKEN")?,
        Some(t) => token = t,
    }
    Ok(token)
}

pub fn get_pulumi_client(
    path: String,
    pulumi_token: Option<String>,
) -> Result<reqwest::RequestBuilder> {
    let token = get_pulumi_token(pulumi_token)?;
    let pulumi_api_endpoint = get_pulumi_endpoint()?;

    let authorization = format!("token {}", token);

    let client = reqwest::Client::new()
        .get(format!("{}/{}/export", pulumi_api_endpoint, path))
        .header("Accept", "application/vnd.pulumi+8")
        .header("Content-Type", "application/json")
        .header("Authorization", authorization);

    Ok(client)
}

/// get the output value from the cloudformation stack
pub async fn get_pulumi_output(
    path: String,
    remote_path: String,
    pulumi_token: Option<String>,
) -> Result<String> {
    let outputs = get_pulumi_outputs(path, pulumi_token).await?;
    let result = outputs
        .dot_get::<serde_json::Value>(remote_path.as_ref())
        .unwrap()
        .unwrap();

    Ok(result.to_string())
}

// get the secret data from the whole outputs of the pulumi stack
pub async fn get_pulumi_outputs_as_secret_data(
    path: String,
    pulumi_token: Option<String>,
) -> Result<BTreeMap<String, ByteString>> {
    let outputs = get_pulumi_outputs(path, pulumi_token).await?;
    let mut secrets = BTreeMap::new();
    for (key, value) in outputs.as_object().unwrap() {
        let remote_path = key.to_string();
        let value_string = value.to_string();
        let output_value = ByteString(value_string.as_bytes().to_vec());
        secrets.insert(remote_path, output_value);
    }

    Ok(secrets)
}

mod tests {
    #![allow(unused_imports)]
    use super::*;

    #[tokio::test]
    async fn test_get_vault_value() {
        if std::env::var("PULUMI_ACCESS_TOKEN").is_ok() {
            let result2 = get_pulumi_outputs("sharonlucky11/test/dev".to_string(), None).await;

            println!("{:?}", result2);
        }
    }
}
