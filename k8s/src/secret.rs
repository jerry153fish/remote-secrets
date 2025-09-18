use crd::{Backend, BackendType, RSecret, RemoteValue, SecretData};

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
use plugins::vault::Vault;
use serde_json::{json, Value};
use std::collections::{hash_map::DefaultHasher, BTreeMap};
use std::hash::{Hash, Hasher};
use utils::value::{get_json_string_nested_value, merge_secret_data};

pub async fn collect_secret_data(rsecret: &RSecret) -> BTreeMap<String, ByteString> {
    let mut secrets = BTreeMap::new();

    for backend in rsecret.spec.resources.iter() {
        let backend_data = resolve_backend_data(backend).await;
        secrets = merge_secret_data(backend_data, secrets);
    }

    secrets
}

async fn resolve_backend_data(backend: &Backend) -> BTreeMap<String, ByteString> {
    match backend.backend {
        BackendType::Plaintext => PlainText::from_backend(backend).get_value().await,
        BackendType::SecretManager => SecretManager::from_backend(backend).get_value().await,
        BackendType::SSM => SSM::from_backend(backend).get_value().await,
        BackendType::Cloudformation => Cloudformation::from_backend(backend).get_value().await,
        BackendType::Pulumi => Pulumi::from_backend(backend).get_value().await,
        BackendType::Vault => Vault::from_backend(backend).get_value().await,
        _ => BTreeMap::new(),
    }
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
pub async fn create_k8s_secret(
    client: Client,
    rsecret: &RSecret,
    data: &BTreeMap<String, ByteString>,
) -> Result<Secret, kube::Error> {
    let name = rsecret.metadata.name.clone().unwrap_or_default();
    let ns = rsecret
        .metadata
        .namespace
        .clone()
        .unwrap_or_else(|| "default".to_owned());

    let hash_id = calculate_secret_hash(data);
    let labels = build_labels(&name, hash_id);

    let k8s_secret: Secret = Secret {
        metadata: ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            labels: Some(labels),
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
    data: &BTreeMap<String, ByteString>,
) -> Result<Secret, kube::Error> {
    let name = rsecret.metadata.name.clone().unwrap_or_default();
    let ns = rsecret
        .metadata
        .namespace
        .clone()
        .unwrap_or_else(|| "default".to_owned());

    let k8s_secret_api: Api<Secret> = Api::namespaced(client.clone(), &ns);

    if k8s_secret_api.get(&name).await.is_ok() {
        let hash_id = calculate_secret_hash(data);

        let data_value = serde_json::to_value(data).map_err(kube::Error::SerdeError)?;

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
        create_k8s_secret(client.clone(), rsecret, data).await
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

pub fn calculate_secret_hash(data: &BTreeMap<String, ByteString>) -> u64 {
    let mut hasher = DefaultHasher::new();
    for (key, value) in data {
        key.hash(&mut hasher);
        hasher.write(&value.0);
    }
    hasher.finish()
}

/// get the hash id from the k8s secret
pub fn get_hash_id(secret: &Secret) -> Option<u64> {
    let labels = secret.metadata.labels.as_ref()?;
    let hash_id = labels.get("hash_id")?;
    hash_id.parse().ok()
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

fn build_labels(name: &str, hash_id: u64) -> BTreeMap<String, String> {
    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("app".to_owned(), name.to_owned());
    labels.insert("hash_id".to_owned(), hash_id.to_string());
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crd::RSecretdSpec;
    use kube::core::ObjectMeta;

    fn sample_rsecret() -> RSecret {
        let backend = Backend {
            backend: BackendType::Plaintext,
            data: vec![SecretData {
                value: "plain-value".into(),
                is_json_string: None,
                remote_path: None,
                key: Some("plain-key".into()),
                configuration_profile_id: None,
                version_number: None,
            }],
            pulumi_token: None,
        };

        let spec = RSecretdSpec {
            resources: vec![backend],
            description: None,
        };

        let mut rsecret = RSecret::new("example", spec);
        rsecret.metadata.namespace = Some("default".into());
        rsecret
    }

    #[tokio::test]
    async fn collects_plaintext_secret_data() {
        let rsecret = sample_rsecret();
        let data = collect_secret_data(&rsecret).await;
        assert_eq!(data.len(), 1);
        let value = data.get("plain-key").expect("missing key");
        assert_eq!(value.0.as_slice(), b"plain-value");
    }

    #[test]
    fn parses_hash_id_from_secret_labels() {
        let mut labels = BTreeMap::new();
        labels.insert("hash_id".into(), "42".into());
        let secret = Secret {
            metadata: ObjectMeta {
                labels: Some(labels),
                ..ObjectMeta::default()
            },
            ..Secret::default()
        };

        assert_eq!(get_hash_id(&secret), Some(42));
    }

    #[test]
    fn returns_none_when_hash_label_missing() {
        let secret = Secret {
            metadata: ObjectMeta::default(),
            ..Secret::default()
        };

        assert_eq!(get_hash_id(&secret), None);
    }

    #[test]
    fn builds_labels_with_hash_and_app() {
        let labels = build_labels("name", 10);
        assert_eq!(labels.get("app"), Some(&"name".to_string()));
        assert_eq!(labels.get("hash_id"), Some(&"10".to_string()));
    }
}
