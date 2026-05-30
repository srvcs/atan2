pub mod api;
pub mod client;
pub mod config;
pub mod health;
pub mod telemetry;

use axum::{extract::Request, http::Response, routing::get, Router};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::Span;

pub fn service_name() -> &'static str {
    "srvcs-atan2"
}

/// Build application router
pub fn router(metrics: telemetry::MetricsHandle, deps: api::Deps) -> Router {
    let trace = TraceLayer::new_for_http()
        .make_span_with(|req: &Request| {
            let request_id = req
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            tracing::info_span!(
                "request",
                method = %req.method(),
                path = %req.uri().path(),
                service = "srvcs-atan2",
                version = env!("CARGO_PKG_VERSION"),
                request_id = %request_id,
            )
        })
        .on_response(|res: &Response<_>, latency: Duration, _span: &Span| {
            tracing::info!(
                status = res.status().as_u16(),
                latency_ms = latency.as_millis() as u64,
                "response"
            );
        });

    Router::new()
        .route("/", get(api::index).post(api::evaluate))
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .route("/openapi.json", get(api::openapi_json))
        .route(
            "/metrics",
            get({
                let h = metrics.clone();
                move || {
                    let h = h.clone();
                    async move { h.render() }
                }
            }),
        )
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(trace)
                .layer(PropagateRequestIdLayer::x_request_id())
                .layer(CatchPanicLayer::new()),
        )
        .with_state(deps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn test_deps() -> api::Deps {
        api::Deps {
            isnumber_url: "http://127.0.0.1:1".to_string(),
        }
    }

    #[test]
    fn service_name_is_stable() {
        assert_eq!(service_name(), "srvcs-atan2");
    }

    #[tokio::test]
    async fn metrics_route_responds() {
        let app = router(telemetry::metrics_handle_for_tests(), test_deps());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
