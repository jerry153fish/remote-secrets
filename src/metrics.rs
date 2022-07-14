use prometheus::{
    default_registry, proto::MetricFamily, register_histogram_vec, register_int_counter,
    HistogramOpts, HistogramVec, IntCounter,
};

/// Metrics exposed on /metrics
#[derive(Clone)]
pub struct Metrics {
    pub reconciliations: IntCounter,
    pub failures: IntCounter,
    pub create_counts: IntCounter,
    pub update_counts: IntCounter,
    pub reconcile_duration: HistogramVec,
}

impl Metrics {
    pub fn new() -> Self {
        let reconcile_histogram = register_histogram_vec!(
            "rsecrets_controller_reconcile_duration_seconds",
            "The duration of reconcile to complete in seconds",
            &[],
            vec![0.01, 0.1, 0.25, 0.5, 1., 5., 15., 60.]
        )
        .unwrap();

        Metrics {
            reconciliations: register_int_counter!(
                "rsecrets_controller_reconciliations_total",
                "reconciliations"
            )
            .unwrap(),
            failures: register_int_counter!(
                "rsecrets_controller_reconciliation_errors_total",
                "reconciliation errors"
            )
            .unwrap(),
            reconcile_duration: reconcile_histogram,
            create_counts: register_int_counter!(
                "rsecrets_controller_create_counts_total",
                "create counts"
            )
            .unwrap(),
            update_counts: register_int_counter!(
                "rsecrets_controller_update_counts_total",
                "update counts"
            )
            .unwrap(),
        }
    }
}
