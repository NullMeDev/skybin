use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;

pub mod handlers;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<crate::db::Database>>,
    pub url_scraper: Option<Arc<crate::scrapers::ExternalUrlScraper>>,
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
        // HTML pages (serve static files)
        .route("/", get(handlers::serve_index))
        .route("/search", get(handlers::serve_search))
        .route("/upload", get(handlers::serve_upload))
        .route("/changelog", get(handlers::serve_changelog))
        .route("/paste/:id", get(handlers::serve_paste))
        .route("/raw/:id", get(handlers::raw_paste))
        // Static assets
        .nest_service("/static", ServeDir::new("static"))
        // API endpoints
        .route("/api/pastes", get(handlers::get_pastes))
        .route("/api/search", get(handlers::search_api))
        .route("/api/stats", get(handlers::statistics))
        .route("/api/health", get(health_check))
        .route("/api/paste/:id", get(handlers::get_paste_api))
        // POST endpoints
        .route("/api/paste", post(handlers::upload_paste_json))
        .route("/api/upload", post(handlers::upload_paste_json))
        .route("/api/submit-url", post(handlers::submit_url))
        // Comments
        .route("/api/paste/:id/comments", get(handlers::get_comments).post(handlers::add_comment))
        // Export
        .route("/api/export/:id/json", get(handlers::export_json))
        .route("/api/export/:id/csv", get(handlers::export_csv))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB limit
        .layer(CompressionLayer::new())
        .with_state(state)
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database: String,
    pub url_queue_size: usize,
    pub timestamp: i64,
}

async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    // Check database connectivity
    let db_status = match state.db.lock() {
        Ok(db) => match db.get_paste_count() {
            Ok(_) => "connected".to_string(),
            Err(e) => format!("error: {}", e),
        },
        Err(_) => "lock_error".to_string(),
    };

    // Get URL queue size if available
    let queue_size = state
        .url_scraper
        .as_ref()
        .map(|s| s.queue_size())
        .unwrap_or(0);

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_status,
        url_queue_size: queue_size,
        timestamp: chrono::Utc::now().timestamp(),
    })
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
