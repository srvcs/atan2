use srvcs_atan2::api::ApiDoc;
use utoipa::OpenApi;

#[test]
fn openapi_snapshot_is_current() {
    let current = serde_json::to_string_pretty(&ApiDoc::openapi()).unwrap();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");
    if std::env::var("UPDATE_OPENAPI").is_ok() {
        std::fs::write(path, current + "\n").unwrap();
        return;
    }

    let committed = std::fs::read_to_string(path).expect("openapi.json missing");
    assert_eq!(
        committed.trim(),
        current.trim(),
        "OpenAPI drifted - run: UPDATE_OPENAPI=1 nix develop -c cargo test --test openapi_snapshot"
    );
}
