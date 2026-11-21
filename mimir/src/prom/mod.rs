use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};

pub mod config;
pub mod http;
pub mod lables;
pub mod names;
pub mod server;

const LATENCY_BUCKETS: &[f64] = &[0.005, 0.025, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0];
pub fn init(service: &str) -> PrometheusHandle {
    let handle = PrometheusBuilder::new()
        .add_global_label("service", service)
        .set_buckets_for_metric(Matcher::Suffix("_seconds".into()), LATENCY_BUCKETS)
        .expect("Valid buckets")
        .install_recorder()
        .expect("metrics recorder installed twice");
    names::describe_all();
    handle
}
