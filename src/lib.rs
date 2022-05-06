#![warn(rust_2018_idioms)]
#![allow(unused_imports)]
#![allow(clippy::blacklisted_name)]

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Kube Api Error: {0}")]
    KubeError(#[source] kube::Error),

    #[error("SerializationError: {0}")]
    SerializationError(#[source] serde_json::Error),

    #[error("Environment error: {0}")]
    VarError(#[source] std::env::VarError),

    #[error("reqwest: {0}")]
    ReqwestError(#[source] reqwest::Error),

    #[error("Unknown error: {0}")]
    UnknowError(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// crd
/// Generated type, for crdgen
pub mod crd;
pub use crd::Backend;
pub use crd::BackendType;
pub use crd::RSecret;
pub use crd::RSecretStatus;

pub mod aws;

/// manager
pub mod manager;
pub use manager::Manager;

/// metrics
pub mod metrics;
pub use metrics::Metrics;

pub mod rsecret;
pub mod utils;
pub mod vault;
pub mod web;
