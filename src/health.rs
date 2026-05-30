use axum::http::StatusCode;
use std::sync::atomic::{AtomicBool, Ordering};

static READY: AtomicBool = AtomicBool::new(false);

/// Liveness
pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

pub fn set_ready(ready: bool) {
    READY.store(ready, Ordering::SeqCst);
}

/// Readiness
pub async fn readyz() -> StatusCode {
    if READY.load(Ordering::SeqCst) {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn healthz_is_ok() {
        assert_eq!(healthz().await, StatusCode::OK);
    }

    #[tokio::test]
    async fn readyz_reflects_state() {
        set_ready(false);
        assert_eq!(readyz().await, StatusCode::SERVICE_UNAVAILABLE);
        set_ready(true);
        assert_eq!(readyz().await, StatusCode::OK);
    }
}
