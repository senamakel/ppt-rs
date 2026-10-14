//! Integration tests for the REST/HTTP API (`http` feature).

use axum::body::Body;
use http_body_util::BodyExt;
use ppt_rs::generator::{SlideContent, create_pptx_with_content};
use tower::ServiceExt;

#[tokio::test]
async fn validate_endpoint_accepts_generated_pptx() {
    let bytes = create_pptx_with_content(
        "Validate",
        vec![SlideContent::new("Slide").add_bullet("Point")],
    )
    .unwrap();

    let response = ppt_rs::http::router()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/v1/validate")
                .body(Body::from(bytes))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["valid"], true);
}

#[tokio::test]
async fn markdown_endpoint_returns_pptx() {
    let payload = serde_json::json!({
        "markdown": "# Slide\n\n- Alpha",
        "title": "From HTTP"
    });

    let response = ppt_rs::http::router()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/v1/markdown")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body.starts_with(b"PK"));
}

#[tokio::test]
async fn diff_endpoint_reports_differences() {
    use base64::Engine;

    let old = create_pptx_with_content(
        "Deck",
        vec![SlideContent::new("Slide").add_bullet("One")],
    )
    .unwrap();
    let new = create_pptx_with_content(
        "Deck",
        vec![SlideContent::new("Slide").add_bullet("Two")],
    )
    .unwrap();

    let payload = serde_json::json!({
        "old_pptx": base64::engine::general_purpose::STANDARD.encode(&old),
        "new_pptx": base64::engine::general_purpose::STANDARD.encode(&new),
    });

    let response = ppt_rs::http::router()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/v1/diff")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["equal"], false);
}
