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
    kind = "Rcrd",
    group = "jerry153fish.com",
    version = "v1beta1",
    namespaced
)]
#[kube(status = "RcrdStatus")]
pub struct RcrdSpec {
    name: String,
    info: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct RcrdStatus {
    is_bad: bool,
    //last_updated: Option<DateTime<Utc>>,
}
