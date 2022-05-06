#![warn(rust_2018_idioms)]
#![allow(unused_imports)]
#![allow(clippy::blacklisted_name)]
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
