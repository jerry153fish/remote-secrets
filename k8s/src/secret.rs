use crd::{BackendType, RSecret, RemoteValue, SecretData};

use anyhow::Result;
use k8s_openapi::{api::core::v1::Secret, ByteString};
use kube::{
    api::{DeleteParams, Patch, PatchParams, PostParams},
    core::ObjectMeta,
};
use kube::{Api, Client};
use plugins::aws_cfn::Cloudformation;
use plugins::aws_secret_manager::SecretManager;
use plugins::aws_ssm::SSM;
use plugins::plaintext::PlainText;
use plugins::pulumi::Pulumi;
use serde_json::{json, Value};
use std::collections::{hash_map::DefaultHasher, BTreeMap};
use std::hash::{Hash, Hasher};
use utils::value::{get_json_string_nested_value, merge_secret_data};

pub async fn get_secret_data(
    rsecret: &RSecret,
    data: BTreeMap<String, ByteString>,
) -> BTreeMap<String, ByteString> {
    let mut secrets = BTreeMap::new();

    for backend in rsecret.spec.resources.iter() {
        match backend.backend {
            BackendType::Plaintext => {
                let plain_text_secret_data = PlainText::from_backend(backend).get_value().await;
                secrets = merge_secret_data(plain_text_secret_data, secrets);
            }
            BackendType::SecretManager => {
                let secret_manager_secret_data =
                    SecretManager::from_backend(backend).get_value().await;
                secrets = merge_secret_data(secret_manager_secret_data, secrets);
            }
            BackendType::SSM => {
                let aws_ssm_data = SSM::from_backend(backend).get_value().await;
                secrets = merge_secret_data(aws_ssm_data, secrets);
            }
            BackendType::Cloudformation => {
                let aws_cfn_data = Cloudformation::from_backend(backend).get_value().await;
                secrets = merge_secret_data(aws_cfn_data, secrets);
            }
            BackendType::Pulumi => {
                let pulumi_data = Pulumi::from_backend(backend).get_value().await;
                secrets = merge_secret_data(pulumi_data, secrets);
            }
            _ => {}
        };
    }

    merge_secret_data(secrets, data)
}

/// Adds a finalizer record into an `RSecret` kind of resource. If the finalizer already exists,
/// this action has no effect.
pub async fn add(client: Client, name: &str, namespace: &str) -> Result<RSecret, kube::Error> {
    let api: Api<RSecret> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": ["rsecrets.jerry153fish.com/finalizer"]
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    api.patch(name, &PatchParams::default(), &patch).await
}

/// Removes all finalizers from an `RSecret` resource. If there are no finalizers already, this
/// action has no effect.
pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<RSecret, kube::Error> {
    let api: Api<RSecret> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": null
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    api.patch(name, &PatchParams::default(), &patch).await
}

/// create a new secret from rsecret
pub async fn create_k8s_secret(client: Client, rsecret: &RSecret) -> Result<Secret, kube::Error> {
    let name = rsecret.metadata.name.clone().unwrap_or_default();
    let ns = rsecret
        .metadata
        .namespace
        .clone()
        .unwrap_or_else(|| "default".to_owned());

    let mut labels: BTreeMap<String, String> = BTreeMap::new();

    let mut data: BTreeMap<String, ByteString> = BTreeMap::new();

    data = get_secret_data(rsecret, data).await;

    let data_string = serde_json::to_string(&data).unwrap();

    let hash_id = calculate_hash(&data_string);

    labels.insert("app".to_owned(), name.clone());
    labels.insert("hash_id".to_owned(), hash_id.to_string());

    let k8s_secret: Secret = Secret {
        metadata: ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            labels: Some(labels.clone()),
            ..ObjectMeta::default()
        },
        type_: Some("Opaque".to_owned()),
        data: Some(data.clone()),
        immutable: Some(false),
        ..Secret::default()
    };
    let k8s_secret_api: Api<Secret> = Api::namespaced(client.clone(), &ns);

    k8s_secret_api
        .create(&PostParams::default(), &k8s_secret)
        .await
}

/// update a secret from rsecret
pub async fn update_k8s_secret(
    client: Client,
    rsecret: &RSecret,
    data_string: &str,
) -> Result<Secret, kube::Error> {
    let name = rsecret.metadata.name.clone().unwrap_or_default();
    let ns = rsecret
        .metadata
        .namespace
        .clone()
        .unwrap_or("default".to_owned());

    let k8s_secret_api: Api<Secret> = Api::namespaced(client.clone(), &ns);

    if k8s_secret_api.get(&name).await.is_ok() {
        let hash_id = calculate_hash(&data_string.to_string());

        let data_value = serde_json::from_str::<Value>(data_string).unwrap();

        let secret_patch_value = json!({
            "metadata": {
                "labels": {
                    "hash_id": hash_id.to_string()
                }
            },
            "data": data_value
        });

        let patch: Patch<&Value> = Patch::Merge(&secret_patch_value);
        k8s_secret_api
            .patch(&name, &PatchParams::default(), &patch)
            .await
    } else {
        create_k8s_secret(client.clone(), rsecret).await
    }
}

/// delete a secret by name
pub async fn delete_k8s_secret(
    client: Client,
    name: &str,
    namespace: &str,
) -> Result<(), kube::Error> {
    let api: Api<Secret> = Api::namespaced(client, namespace);
    if api.get(name).await.is_ok() {
        api.delete(name, &DeleteParams::default()).await?;
    }

    Ok(())
}

/// get the hash id from string
pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

/// get the hash id from the k8s secret
pub fn get_hash_id(secret: &Secret) -> String {
    let labels = secret.metadata.labels.as_ref().unwrap();
    let hash_id = labels.get("hash_id").unwrap();
    hash_id.to_string()
}

// TODO: a better way error handling
pub fn rsecret_data_to_secret_data(
    rsecret_data: &SecretData,
    value_string: &str,
) -> BTreeMap<String, ByteString> {
    let mut secrets = BTreeMap::new();
    if rsecret_data.key.is_some() {
        let key = rsecret_data.key.clone().unwrap();

        if rsecret_data.is_json_string.unwrap_or_default() {
            if rsecret_data.remote_path.is_some() {
                let value = get_json_string_nested_value(
                    value_string,
                    &rsecret_data.remote_path.clone().unwrap(),
                );

                match value {
                    Ok(value) => {
                        if !value.is_empty() {
                            secrets.insert(key, ByteString(value.as_bytes().to_vec()));
                        }
                    }
                    Err(e) => {
                        log::error!("{}", e);
                    }
                }
            }
        } else {
            secrets.insert(key, ByteString(value_string.as_bytes().to_vec()));
        }
    }

    secrets
}
