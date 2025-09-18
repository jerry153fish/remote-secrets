use chrono::prelude::*;
use futures::{future::BoxFuture, FutureExt, StreamExt};
use k8s_openapi::api::core::v1::Secret;
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::{
        controller::{Action, Controller},
        events::Reporter,
    },
    Client, Resource,
};
use serde::Serialize;
use std::sync::Arc;
use tokio::{sync::RwLock, time::Duration};

use log::{info, warn};

use crd::RSecret;
use k8s::secret;
use utils::metrics::FAILURES;
use utils::metrics::RECONCILIATIONS;

use anyhow::Result;

async fn reconcile(rsecret: Arc<RSecret>, ctx: Arc<ContextData>) -> Result<Action, kube::Error> {
    // let start = Instant::now();
    RECONCILIATIONS.inc();

    let client = ctx.client.clone();
    ctx.state.write().await.last_event = Utc::now();
    let name = ResourceExt::name_any(rsecret.as_ref());
    let ns = ResourceExt::namespace(rsecret.as_ref()).expect("rsecret is namespaced");

    let rs = rsecret.as_ref().clone();

    // let duration = start.elapsed().as_millis() as f64 / 1000.0;

    // Performs action as decided by the `determine_action` function.
    match determine_action(&rsecret) {
        RSecretAction::Create => {
            secret::add(client.clone(), &name, &ns).await?;

            let data = secret::collect_secret_data(&rs).await;
            secret::create_k8s_secret(client.clone(), &rs, &data).await?;
            // ctx.get_ref().metrics.create_counts.inc();
            Ok(Action::requeue(Duration::from_secs(20)))
        }
        RSecretAction::Delete => {
            secret::delete_k8s_secret(client.clone(), &name, &ns).await?;

            secret::delete(client.clone(), &rsecret.name_any(), &ns).await?;
            Ok(Action::await_change())
        }

        RSecretAction::Update => {
            info!("Updating rsecret {} in namespace {}", name, ns);

            let k8s_secrets: Api<Secret> = Api::namespaced(client.clone(), &ns);
            let secret = k8s_secrets.get(&name).await;

            match secret {
                Ok(secret) => {
                    let data = secret::collect_secret_data(&rsecret).await;
                    let new_hash_id = secret::calculate_secret_hash(&data);
                    let old_hash_id = secret::get_hash_id(&secret);

                    if old_hash_id == Some(new_hash_id) {
                        info!("No changes to rsecret {} in namespace {}", name, ns);
                    } else {
                        info!("Updating rsecret {} in namespace {}", name, ns);
                        secret::update_k8s_secret(client.clone(), &rs, &data).await?;
                        // ctx.get_ref().metrics.update_counts.inc();
                    }
                }
                Err(_error) => {
                    // TODO: sort out the error type
                    secret::add(client.clone(), &name, &ns).await?;

                    let data = secret::collect_secret_data(&rsecret).await;
                    secret::create_k8s_secret(client.clone(), &rs, &data).await?;
                }
            }

            Ok(Action::requeue(Duration::from_secs(20)))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum RSecretAction {
    Create,
    Delete,
    Update,
}

fn determine_action(rsecret: &RSecret) -> RSecretAction {
    if rsecret.meta().deletion_timestamp.is_some() {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crd::RSecretdSpec;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
    use kube::core::ObjectMeta;

    fn base_rsecret() -> RSecret {
        let spec = RSecretdSpec {
            resources: vec![],
            description: None,
        };
        let mut rsecret = RSecret::new("example", spec);
        rsecret.metadata = ObjectMeta {
            namespace: Some("default".into()),
            ..ObjectMeta::default()
        };
        rsecret
    }

    #[test]
    fn determine_action_returns_create_when_no_finalizer() {
        let rsecret = base_rsecret();
        assert_eq!(determine_action(&rsecret), RSecretAction::Create);
    }

    #[test]
    fn determine_action_returns_update_when_finalizer_present() {
        let mut rsecret = base_rsecret();
        rsecret.metadata.finalizers = Some(vec!["rsecrets.jerry153fish.com/finalizer".into()]);
        assert_eq!(determine_action(&rsecret), RSecretAction::Update);
    }

    #[test]
    fn determine_action_returns_delete_when_deletion_timestamp_set() {
        let mut rsecret = base_rsecret();
        rsecret.metadata.deletion_timestamp = Some(Time(Utc::now()));
        assert_eq!(determine_action(&rsecret), RSecretAction::Delete);
    }
}

fn error_policy(_r: Arc<RSecret>, error: &kube::Error, _ctx: Arc<ContextData>) -> Action {
    warn!("reconcile failed: {:?}", error);
    FAILURES.inc();
    Action::requeue(Duration::from_secs(5 * 60))
}

#[derive(Clone)]
pub struct ContextData {
    /// Kubernetes client to make Kubernetes API requests with. Required for K8S resource management.
    client: Client,

    state: Arc<RwLock<State>>,
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

/// Example Manager that owns a Controller for RSecret
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
        let context = Arc::new(ContextData {
            client: client.clone(),
            state: state.clone(),
        });

        // Preparation of resources used by the `kube_runtime::Controller`
        let rsecrets: Api<RSecret> = Api::all(client.clone());

        // Ensure CRD is installed before loop-watching
        let _r = rsecrets.list(&ListParams::default().limit(1)).await.expect(
            "is the crd installed? please run: cargo run --bin crdgen | kubectl apply -f -",
        );

        // All good. Start controller and return its future.
        let drainer = Controller::new(rsecrets.clone(), kube::runtime::watcher::Config::default())
            .run(reconcile, error_policy, context)
            .filter_map(|x| async move { std::result::Result::ok(x) })
            .for_each(|_| futures::future::ready(()))
            .boxed();

        (Self { state }, drainer)
    }

    /// State getter
    pub async fn state(&self) -> State {
        self.state.read().await.clone()
    }
}
