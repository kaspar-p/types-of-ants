use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicI64, Ordering};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Production: returns the real wall-clock time. Zero-sized.
pub struct WallClock;

impl Clock for WallClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Tests: returns a strictly-increasing timestamp on every call (start + N seconds).
/// Wrap in `Arc<dyn Clock>` when storing in state — that Arc handles sharing across
/// clones, so no inner Arc is needed here.
pub struct TestClock(AtomicI64);

impl TestClock {
    pub fn new(start_secs: i64) -> Self {
        Self(AtomicI64::new(start_secs))
    }
}

impl Clock for TestClock {
    fn now(&self) -> DateTime<Utc> {
        let t = self.0.fetch_add(1, Ordering::Relaxed);
        DateTime::from_timestamp(t, 0).unwrap_or_else(Utc::now)
    }
}
