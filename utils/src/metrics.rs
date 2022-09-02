use lazy_static::lazy_static;
use prometheus::{register_int_counter, IntCounter, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref RECONCILIATIONS: IntCounter = register_int_counter!(
        "rsecrets_controller_reconciliations_total",
        "reconciliations"
    )
    .unwrap();
    pub static ref FAILURES: IntCounter = register_int_counter!(
        "rsecrets_controller_reconciliation_errors_total",
        "reconciliation errors"
    )
    .unwrap();
}

pub fn register_custom_metrics() {
    REGISTRY
        .register(Box::new(RECONCILIATIONS.clone()))
        .expect("collector can be registered");

    REGISTRY
        .register(Box::new(FAILURES.clone()))
        .expect("collector can be registered");
}
