use crate::state::AntArchiveStorageState;
use axum::{extract::State, routing::get, Router};
use axum_prometheus::{PrometheusMetricLayer, PrometheusMetricLayerBuilder};
use http::StatusCode;
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::{atomic::Ordering, OnceLock};

static GLOBAL_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

pub fn build_metric_layer() -> (PrometheusMetricLayer<'static>, PrometheusHandle) {
    let handle = GLOBAL_HANDLE.get_or_init(|| {
        let (_, h) = PrometheusMetricLayerBuilder::new()
            .with_prefix("ant_archive_storage")
            .with_default_metrics()
            .build_pair();
        h
    });
    let (layer, _) = PrometheusMetricLayerBuilder::new()
        .with_metrics_from_fn(|| handle.clone())
        .build_pair();
    (layer, handle.clone())
}

async fn metrics_handler(State(state): State<AntArchiveStorageState>) -> (StatusCode, String) {
    let mut output = state.metrics_handle.render();
    let bytes = state.bytes_stored.load(Ordering::Relaxed);
    output.push_str(&format!(
        "# HELP ant_archive_storage_bytes_stored Total logical bytes currently stored\n# TYPE \
         ant_archive_storage_bytes_stored gauge\nant_archive_storage_bytes_stored {bytes}\n"
    ));
    (StatusCode::OK, output)
}

pub fn make_metrics_routes(state: AntArchiveStorageState) -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}
