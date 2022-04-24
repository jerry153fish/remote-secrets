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
use std::{collections::HashMap, sync::Arc};

/// Our Foo custom resource spec
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    kind = "RSecret",
    group = "jerry153fish.com",
    version = "v1beta1",
    namespaced
)]
#[kube(status = "RSecretStatus")]
pub struct RSecretdSpec {
    name: String,
    #[serde(default)]
    resources: Vec<Backend>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct Backend {
    /// Remote backend type
    backend: BackendType,

    /// Secret data configurations
    #[serde(default)]
    data: Vec<SecretData>,

    /// Pulumi secret for the pulumi backend
    #[serde(skip_serializing_if = "Option::is_none")]
    pulumi_token: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct SecretData {
    /// name for the remote backend
    /// for ssm / paramstore / application configuration this is the name or arn
    /// for cloudformation and pulumi this is the stack name
    remote_name: String,

    /// whether the remote data is jsonstrinified string or not
    is_json_string: bool,

    /// nested path for the remote data
    #[serde(skip_serializing_if = "Option::is_none")]
    remote_nest_path: Option<String>,

    /// secret field name
    #[serde(skip_serializing_if = "Option::is_none")]
    secret_field_name: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
enum BackendType {
    SSM,
    SecretManager,
    Cloudformation,
    AppConfig,
    Pulumi,
    Plaintext,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct RSecretStatus {
    is_bad: bool,
    //last_updated: Option<DateTime<Utc>>,
}
