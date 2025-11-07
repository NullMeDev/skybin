use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tower_http::compression::CompressionLayer;

pub mod handlers;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<crate::db::Database>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: String) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

#[derive(Debug)]
pub struct ApiError(pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::err(self.0)),
        )
            .into_response()
    }
}

/// Create the Axum router with all routes
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // HTML pages
        .route("/", get(handlers::feed))
        .route("/search", get(handlers::search))
        .route("/upload", get(handlers::upload_page))
        .route("/paste/:id", get(handlers::view_paste))
        .route("/raw/:id", get(handlers::raw_paste))
        
        // API endpoints
        .route("/api/pastes", get(handlers::get_pastes))
        .route("/api/search", get(handlers::search_api))
        .route("/api/stats", get(handlers::statistics))
        .route("/api/health", get(health_check))
        .route("/api/paste/:id", get(handlers::view_paste))
        
        // POST endpoints
        .route("/api/upload", post(handlers::upload_paste_json))
        
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB limit
        .layer(CompressionLayer::new())
        .with_state(state)
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_ok() {
        let response: ApiResponse<String> = ApiResponse::ok("test".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("test".to_string()));
        assert_eq!(response.error, None);
    }

    #[test]
    fn test_api_response_err() {
        let response: ApiResponse<String> = ApiResponse::err("error".to_string());
        assert!(!response.success);
        assert_eq!(response.data, None);
        assert_eq!(response.error, Some("error".to_string()));
    }
}
