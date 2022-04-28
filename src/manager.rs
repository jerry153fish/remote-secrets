use chrono::prelude::*;
use futures::{future::BoxFuture, FutureExt, StreamExt};
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::ByteString;
use kube::{
    api::{Api, DeleteParams, ListParams, ObjectMeta, Patch, PatchParams, PostParams, ResourceExt},
    runtime::{
        controller::{Action, Context, Controller},
        events::{Event, EventType, Recorder, Reporter},
    },
    Client, CustomResource, Resource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::RwLock,
    time::{Duration, Instant},
};

use serde_json::{Map, Value};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use prometheus::{default_registry, labels, proto::MetricFamily};

use log::{info, warn, LevelFilter};

use crate::{rsecret, Metrics, RSecret, RSecretStatus};

use std::collections::BTreeMap;

async fn reconcile(
    rsecret: Arc<RSecret>,
    ctx: Context<ContextData>,
) -> Result<Action, kube::Error> {
    let start = Instant::now();
    ctx.get_ref().metrics.reconciliations.inc();

    let client = ctx.get_ref().client.clone();
    ctx.get_ref().state.write().await.last_event = Utc::now();
    let name = ResourceExt::name(rsecret.as_ref());
    let ns = ResourceExt::namespace(rsecret.as_ref()).expect("rsecret is namespaced");

    let rs = rsecret.as_ref().clone();

    let duration = start.elapsed().as_millis() as f64 / 1000.0;
    ctx.get_ref()
        .metrics
        .reconcile_duration
        .with_label_values(&[])
        .observe(duration);

    // Performs action as decided by the `determine_action` function.
    return match determine_action(&rsecret) {
        RSecretAction::Create => {
            rsecret::add(client.clone(), &name, &ns).await?;

            rsecret::create_k8s_secret(client.clone(), &rs).await?;
            Ok(Action::requeue(Duration::from_secs(10)))
        }
        RSecretAction::Delete => {
            rsecret::delete_k8s_secret(client.clone(), &name, &ns).await?;

            rsecret::delete(client.clone(), &rsecret.name(), &ns).await?;
            Ok(Action::await_change())
        }

        RSecretAction::Update => {
            info!("Updating rsecret {} in namespace {}", name, ns);

            let k8s_secrets: Api<Secret> = Api::namespaced(client.clone(), &ns);
            let secret = k8s_secrets.get(&name).await?;
            let old_hash_id = rsecret::get_hash_id(&secret);

            let mut data: BTreeMap<String, ByteString> = BTreeMap::new();
            data = rsecret::get_secret_data(&rsecret, data).await;
            let data_string = serde_json::to_string(&data).unwrap();
            let new_hash_id = rsecret::calculate_hash(&data_string);

            if new_hash_id.to_string().eq(&old_hash_id) {
                info!("No changes to rsecret {} in namespace {}", name, ns);
            } else {
                info!("Updating rsecret {} in namespace {}", name, ns);
                rsecret::update_k8s_secret(client.clone(), &rs, data_string.clone()).await?;
            }

            Ok(Action::requeue(Duration::from_secs(10)))
        }
    };
}

enum RSecretAction {
    Create,
    Delete,
    Update,
}

fn determine_action(rsecret: &RSecret) -> RSecretAction {
    return if rsecret.meta().deletion_timestamp.is_some() {
        RSecretAction::Delete
    } else if rsecret
        .meta()
        .finalizers
        .as_ref()
        .map_or(true, |finalizers| finalizers.is_empty())
    {
        RSecretAction::Create
    } else {
        RSecretAction::Update
    };
}

fn error_policy(error: &kube::Error, ctx: Context<ContextData>) -> Action {
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
