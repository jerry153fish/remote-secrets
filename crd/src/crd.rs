use anyhow::Result;
use async_trait::async_trait;
use chrono::prelude::*;
use k8s_openapi::ByteString;
use kube::{
    api::{Api, ListParams, Patch, PatchParams, ResourceExt},
    client::Client,
    runtime::{
        controller::{Action, Context, Controller},
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
    /// for ssm / paramstore this is the name of the key
    /// for cloudformation and pulumi this is the stack name
    /// for plaintext this is the value of the secret
    /// for appconfig this is the application id
    pub remote_value: String,

    /// whether the remote data is jsonstrinified string or not
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_json_string: Option<bool>,

    /// nested path for the remote data, if remote value is a json
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_nest_path: Option<String>,

    /// secret field name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_field_name: Option<String>,

    /// output key for cloudformation or pulumi
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_key: Option<String>,

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
