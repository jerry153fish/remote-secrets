use lazy_static::lazy_static;
use prometheus::{
    register_int_counter, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, Opts, Registry,
};

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
    pub static ref SYNC_ATTEMPTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "rsecrets_controller_sync_attempts_total",
            "Total secret sync attempts partitioned by action and result",
        ),
        &["action", "result"],
    )
    .unwrap();
    pub static ref SYNC_SUCCESS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "rsecrets_controller_sync_success_total",
            "Successful secret sync operations partitioned by action",
        ),
        &["action"],
    )
    .unwrap();
    pub static ref SYNC_FAILURE_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "rsecrets_controller_sync_failure_total",
            "Failed secret sync operations partitioned by action",
        ),
        &["action"],
    )
    .unwrap();
    pub static ref SYNC_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "rsecrets_controller_sync_duration_seconds",
            "Secret sync operation duration in seconds partitioned by action and result",
        ),
        &["action", "result"],
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

    REGISTRY
        .register(Box::new(SYNC_ATTEMPTS_TOTAL.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(SYNC_SUCCESS_TOTAL.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(SYNC_FAILURE_TOTAL.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(SYNC_DURATION_SECONDS.clone()))
        .expect("collector can be registered");
}

pub fn record_sync_metrics(action: &str, result: &str, duration_seconds: f64) {
    SYNC_ATTEMPTS_TOTAL
        .with_label_values(&[action, result])
        .inc();
    match result {
        "success" => SYNC_SUCCESS_TOTAL.with_label_values(&[action]).inc(),
        "failure" => SYNC_FAILURE_TOTAL.with_label_values(&[action]).inc(),
        _ => {}
    }
    SYNC_DURATION_SECONDS
        .with_label_values(&[action, result])
        .observe(duration_seconds);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_sync_metrics_increments_counters_and_duration() {
        let attempts_before = SYNC_ATTEMPTS_TOTAL
            .with_label_values(&["update", "success"])
            .get();
        let success_before = SYNC_SUCCESS_TOTAL.with_label_values(&["update"]).get();
        let duration_before = SYNC_DURATION_SECONDS
            .with_label_values(&["update", "success"])
            .get_sample_count();

        record_sync_metrics("update", "success", 0.123);

        let attempts_after = SYNC_ATTEMPTS_TOTAL
            .with_label_values(&["update", "success"])
            .get();
        let success_after = SYNC_SUCCESS_TOTAL.with_label_values(&["update"]).get();
        let duration_after = SYNC_DURATION_SECONDS
            .with_label_values(&["update", "success"])
            .get_sample_count();

        assert_eq!(attempts_after, attempts_before + 1);
        assert_eq!(success_after, success_before + 1);
        assert_eq!(duration_after, duration_before + 1);
    }
}
