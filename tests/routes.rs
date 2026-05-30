use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router as AxumRouter};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_atan2::{api::Deps, health, router, telemetry};
use tower::ServiceExt;

/// Spin up a mock `srvcs-isnumber` that genuinely computes "is this a number"
/// from the request body (`{"value": ...}`), answering `200 {"result": <bool>}`.
/// Lets us test orchestration without the real fleet.
async fn spawn_isnumber() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|Json(body): Json<Value>| async move {
            let is_number = body.get("value").map(Value::is_number).unwrap_or(false);
            (StatusCode::OK, Json(json!({ "result": is_number })))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

fn app(isnumber_url: &str) -> axum::Router {
    router(
        telemetry::metrics_handle_for_tests(),
        Deps {
            isnumber_url: isnumber_url.to_string(),
        },
    )
}

async fn eval(isnumber_url: &str, y: Value, x: Value) -> (StatusCode, Value) {
    let res = app(isnumber_url)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "y": y, "x": x }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

// A base URL with nothing listening — exercises the degraded path.
const DEAD_URL: &str = "http://127.0.0.1:1";

async fn status_of(uri: &str) -> StatusCode {
    app(DEAD_URL)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn index_ok() {
    assert_eq!(status_of("/").await, StatusCode::OK);
}

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn metrics_ok() {
    assert_eq!(status_of("/metrics").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let res = app(DEAD_URL)
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        res.headers().contains_key("x-request-id"),
        "response must carry a generated x-request-id"
    );
}

fn approx(body: &Value, expected: f64) {
    let got = body["result"]
        .as_f64()
        .expect("result must be a JSON number (f64)");
    assert!(
        (got - expected).abs() < 1e-9,
        "got {got}, expected {expected}"
    );
}

#[tokio::test]
async fn computes_atan2_per_spec() {
    let isnumber = spawn_isnumber().await;

    // The spec's asserted case: atan2(1, 1) == 0.7853981633974483 == FRAC_PI_4.
    let (status, body) = eval(&isnumber, json!(1), json!(1)).await;
    assert_eq!(status, StatusCode::OK);
    approx(&body, std::f64::consts::FRAC_PI_4);
}

#[tokio::test]
async fn computes_atan2_for_more_quadrants() {
    let isnumber = spawn_isnumber().await;

    let (_, body) = eval(&isnumber, json!(0), json!(1)).await;
    approx(&body, 0.0);

    let (_, body) = eval(&isnumber, json!(1), json!(0)).await;
    approx(&body, std::f64::consts::FRAC_PI_2);

    let (_, body) = eval(&isnumber, json!(0), json!(-1)).await;
    approx(&body, std::f64::consts::PI);
}

#[tokio::test]
async fn accepts_float_operands() {
    let isnumber = spawn_isnumber().await;

    // Floats are valid input for a floating-point service.
    let (status, body) = eval(&isnumber, json!(0.5), json!(0.5)).await;
    assert_eq!(status, StatusCode::OK);
    approx(&body, std::f64::consts::FRAC_PI_4);
}

#[tokio::test]
async fn rejects_when_y_is_not_a_number() {
    let isnumber = spawn_isnumber().await;
    let (status, _) = eval(&isnumber, json!("nope"), json!(1)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rejects_when_x_is_not_a_number() {
    let isnumber = spawn_isnumber().await;
    let (status, _) = eval(&isnumber, json!(1), json!("nope")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn degrades_when_isnumber_is_unreachable() {
    let (status, body) = eval(DEAD_URL, json!(1), json!(1)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-isnumber");
}
