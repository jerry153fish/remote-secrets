use crate::RSecret;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client, Error};
use serde_json::{json, Value};

/// Adds a finalizer record into an `RSecret` kind of resource. If the finalizer already exists,
/// this action has no effect.
pub async fn add(client: Client, name: &str, namespace: &str) -> Result<RSecret, Error> {
    let api: Api<RSecret> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": ["rsecrets.jerry153fish.com/finalizer"]
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    Ok(api.patch(name, &PatchParams::default(), &patch).await?)
}

/// Removes all finalizers from an `RSecret` resource. If there are no finalizers already, this
/// action has no effect.
pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<RSecret, Error> {
    let api: Api<RSecret> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": null
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    Ok(api.patch(name, &PatchParams::default(), &patch).await?)
}
