use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::Arc;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub type MetricsHandle = Arc<PrometheusHandle>;

/// Initialize JSON logging
pub fn init(default_level: &str) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json())
        .try_init();
}

/// Install global recorder
pub fn install_metrics() -> MetricsHandle {
    Arc::new(
        PrometheusBuilder::new()
            .install_recorder()
            .expect("install recorder"),
    )
}

/// Build metrics handle
pub fn metrics_handle_for_tests() -> MetricsHandle {
    Arc::new(PrometheusBuilder::new().build_recorder().handle())
}
