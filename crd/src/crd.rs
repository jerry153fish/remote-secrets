use anyhow::Result;
use async_trait::async_trait;
use chrono::prelude::*;
use k8s_openapi::ByteString;
use kube::{
    api::{Api, ListParams, Patch, PatchParams, ResourceExt},
    client::Client,
    runtime::{
        controller::{Action, Controller},
        events::{Event, EventType, Recorder, Reporter},
    },
    CustomResource, Resource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

/// Our RSecret custom resource spec
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    kind = "RSecret",
    group = "jerry153fish.com",
    version = "v1beta1",
    namespaced
)]
#[kube(status = "RSecretStatus")]
pub struct RSecretdSpec {
    #[serde(default)]
    pub resources: Vec<Backend>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct Backend {
    /// Remote backend type
    pub backend: BackendType,

    /// Secret data configurations
    #[serde(default)]
    pub data: Vec<SecretData>,

    /// Pulumi secret for the pulumi backend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pulumi_token: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct SecretData {
    /// remote value of the backend
    /// for ssm / parameter store / vault: name of the key
    /// for cloudformation and pulumi: stack name
    /// for plaintext: value of the secret
    /// for appconfig: application id
    /// for pulumi: full stack path eg pulumiOriginId/projectName/stackName
    pub value: String,

    /// whether the remote data is jsonstrinified string or not
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_json_string: Option<bool>,

    /// path for the remote data, if remote value is a json
    /// for cloudformation and pulumi should be the outputs path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_path: Option<String>,

    /// secret field name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// configuration profile id for appconfig
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration_profile_id: Option<String>,

    /// version number for the Hosted configuration versions for appconfig
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_number: Option<i32>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum BackendType {
    SSM,
    SecretManager,
    Cloudformation,
    AppConfig,
    Pulumi,
    Plaintext,
    Vault,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct RSecretStatus {
    pub last_updated: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait RemoteValue {
    async fn get_value(&self) -> BTreeMap<String, ByteString>;

    fn from_backend(backend: &Backend) -> Self;
}
