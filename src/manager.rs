use chrono::prelude::*;
use futures::{future::BoxFuture, FutureExt, StreamExt};
use k8s_openapi::api::core::v1::Secret;
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::{
        controller::{Action, Context, Controller},
        events::{Event, EventType, Recorder, Reporter},
    },
    Client, CustomResource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::RwLock,
    time::{Duration, Instant},
};

use prometheus::{default_registry, proto::MetricFamily};

use log::{info, warn, LevelFilter};

use crate::{Error, Metrics, RSecret};

async fn reconcile(rsecret: Arc<RSecret>, context: Context<ContextData>) -> Result<Action, Error> {
    Ok(Action::requeue(Duration::from_secs(10)))
}

fn error_policy(error: &Error, ctx: Context<ContextData>) -> Action {
    warn!("reconcile failed: {:?}", error);
    ctx.get_ref().metrics.failures.inc();
    Action::requeue(Duration::from_secs(5 * 60))
}

#[derive(Clone)]
pub struct ContextData {
    /// Kubernetes client to make Kubernetes API requests with. Required for K8S resource management.
    client: Client,

    state: Arc<RwLock<State>>,

    /// Various prometheus metrics
    metrics: Metrics,
}

/// In-memory reconciler state exposed on /
#[derive(Clone, Serialize)]
pub struct State {
    #[serde(deserialize_with = "from_ts")]
    pub last_event: DateTime<Utc>,
    #[serde(skip)]
    pub reporter: Reporter,
}
impl State {
    fn new() -> Self {
        State {
            last_event: Utc::now(),
            reporter: "rsecrets-controller".into(),
        }
    }
}

/// Data owned by the Manager
#[derive(Clone)]
pub struct Manager {
    /// In memory state
    state: Arc<RwLock<State>>,
}

/// Example Manager that owns a Controller for Foo
impl Manager {
    /// Lifecycle initialization interface for app
    ///
    /// This returns a `Manager` that drives a `Controller` + a future to be awaited
    /// It is up to `main` to wait for the controller stream.
    pub async fn new() -> (Self, BoxFuture<'static, ()>) {
        let client = Client::try_default()
            .await
            .expect("Expected a valid KUBECONFIG environment variable.");

        let state = Arc::new(RwLock::new(State::new()));
        let metrics = Metrics::new();
        let context = Context::new(ContextData {
            client: client.clone(),
            metrics: metrics.clone(),
            state: state.clone(),
        });

        // Preparation of resources used by the `kube_runtime::Controller`
        let rsecrets: Api<RSecret> = Api::all(client.clone());

        // Ensure CRD is installed before loop-watching
        let _r = rsecrets.list(&ListParams::default().limit(1)).await.expect(
            "is the crd installed? please run: cargo run --bin crdgen | kubectl apply -f -",
        );

        // All good. Start controller and return its future.
        let drainer = Controller::new(rsecrets.clone(), ListParams::default())
            .run(reconcile, error_policy, context)
            .filter_map(|x| async move { std::result::Result::ok(x) })
            .for_each(|_| futures::future::ready(()))
            .boxed();

        (Self { state }, drainer)
    }

    /// Metrics getter
    pub fn metrics(&self) -> Vec<MetricFamily> {
        default_registry().gather()
    }

    /// State getter
    pub async fn state(&self) -> State {
        self.state.read().await.clone()
    }
}