use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

use crate::client::{self, DepError};

pub const SERVICE: &str = "srvcs-atan2";
pub const CONCERN: &str = "trigonometry: atan2(y, x)";
pub const DEPENDS_ON: &[&str] = &["srvcs-isnumber"];

/// Dependency endpoints, injected as router state so tests can point them at
/// mock services.
#[derive(Clone)]
pub struct Deps {
    pub isnumber_url: String,
}

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    #[schema(value_type = Object)]
    pub y: Value,
    #[schema(value_type = Object)]
    pub x: Value,
}

#[derive(Serialize, ToSchema)]
pub struct Atan2Response {
    #[schema(value_type = Object)]
    pub y: Value,
    #[schema(value_type = Object)]
    pub x: Value,
    pub result: f64,
}

/// The single concern: the four-quadrant arctangent of `y / x`, in radians.
///
/// Both operands are real numbers (integers or floats are accepted), so
/// `atan2(1.0, 1.0) == FRAC_PI_4` and `atan2(0.0, 1.0) == 0.0`.
pub fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

fn ok(y: Value, x: Value, result: f64) -> Response {
    (
        StatusCode::OK,
        Json(json!({ "y": y, "x": x, "result": result })),
    )
        .into_response()
}

fn invalid(reason: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "error": reason })),
    )
        .into_response()
}

fn degraded(dependency: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "dependency unavailable", "dependency": dependency })),
    )
        .into_response()
}

/// Forward a dependency's response verbatim (used to propagate `422` for invalid
/// input, so atan2 reports the same rejection its dependency did).
fn forward(status: u16, body: Value) -> Response {
    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    (code, Json(body)).into_response()
}

/// Validate a single operand is a number by asking `srvcs-isnumber`, mapping its
/// failures to the response this service should return.
async fn ask_is_number(url: &str, value: &Value, dependency: &str) -> Result<(), Response> {
    match client::call(url, &json!({ "value": value })).await {
        Err(DepError::Unreachable) => Err(degraded(dependency)),
        Ok((200, body)) => {
            let is_number = body.get("result").and_then(Value::as_bool).unwrap_or(false);
            if is_number {
                Ok(())
            } else {
                Err(invalid("value is not a number"))
            }
        }
        // Invalid input propagates from the leaf dependency; forward it.
        Ok((422, body)) => Err(forward(422, body)),
        Ok(_) => Err(degraded(dependency)),
    }
}

/// `POST /` — compute `atan2(y, x)`.
///
/// Input validation for both operands is delegated to `srvcs-isnumber` over HTTP
/// (the single source of truth for "is this a number"), once per operand. Both
/// integers and floats are valid input — this is a floating-point service. If
/// the dependency is unreachable, this service reports itself degraded rather
/// than guessing.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = Atan2Response),
        (status = 422, description = "an operand is not a number"),
        (status = 500, description = "an operand passed validation but is not representable as a number"),
        (status = 503, description = "a dependency is unavailable")
    )
)]
pub async fn evaluate(State(deps): State<Deps>, Json(req): Json<EvalRequest>) -> Response {
    // 1. Delegate "is this a number" to srvcs-isnumber, once per operand.
    if let Err(resp) = ask_is_number(&deps.isnumber_url, &req.y, "srvcs-isnumber").await {
        return resp;
    }
    if let Err(resp) = ask_is_number(&deps.isnumber_url, &req.x, "srvcs-isnumber").await {
        return resp;
    }

    // 2. atan2 accepts any real numbers; coerce both operands to f64.
    let (Some(y), Some(x)) = (req.y.as_f64(), req.x.as_f64()) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                json!({ "error": "operand validated as a number but is not representable as f64" }),
            ),
        )
            .into_response();
    };

    ok(req.y, req.x, atan2(y, x))
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, Atan2Response))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some());
        assert!(root.post.is_some());
    }

    #[test]
    fn atan2_matches_spec() {
        // The spec's asserted value, 0.7853981633974483, is exactly FRAC_PI_4.
        assert!((atan2(1.0, 1.0) - std::f64::consts::FRAC_PI_4).abs() < 1e-9);
        assert!((atan2(0.0, 1.0) - 0.0).abs() < 1e-9);
        assert!((atan2(1.0, 0.0) - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!((atan2(-1.0, 1.0) - (-std::f64::consts::FRAC_PI_4)).abs() < 1e-9);
        assert!((atan2(0.0, -1.0) - std::f64::consts::PI).abs() < 1e-9);
    }

    #[test]
    fn atan2_accepts_whole_and_fractional_inputs() {
        // integers (as f64) and fractional values both compute.
        assert!((atan2(2.0, 2.0) - std::f64::consts::FRAC_PI_4).abs() < 1e-9);
        assert!((atan2(0.5, 0.5) - std::f64::consts::FRAC_PI_4).abs() < 1e-9);
    }

    #[tokio::test]
    async fn index_reports_dependency() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-atan2");
        assert_eq!(info.concern, "trigonometry: atan2(y, x)");
        assert_eq!(info.depends_on, vec!["srvcs-isnumber"]);
    }
}
