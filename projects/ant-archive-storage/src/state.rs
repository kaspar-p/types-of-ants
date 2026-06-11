use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};

use metrics_exporter_prometheus::PrometheusHandle;

#[derive(Clone)]
pub struct AntArchiveStorageState {
    pub root: PathBuf,
    pub metrics_handle: Arc<PrometheusHandle>,
    pub bytes_stored: Arc<AtomicI64>,
}

impl AntArchiveStorageState {
    pub fn new(root: PathBuf, metrics_handle: PrometheusHandle) -> Self {
        AntArchiveStorageState {
            root,
            metrics_handle: Arc::new(metrics_handle),
            bytes_stored: Arc::new(AtomicI64::new(0)),
        }
    }

    pub fn adjust_bytes(&self, delta: i64) {
        self.bytes_stored.fetch_add(delta, Ordering::Relaxed);
    }
}
