use axum::{
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

pub mod handlers;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<crate::db::Database>>,
    pub url_scraper: Option<Arc<crate::scrapers::ExternalUrlScraper>>,
    pub admin: Option<Arc<crate::admin::AdminAuth>>,
    pub rate_limiters: Arc<crate::rate_limiter::ApiRateLimiters>,
    pub config: Arc<crate::config::Config>,
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
    // Configure upload body limit from config (default 400MB)
    let upload_limit = state
        .config
        .server
        .max_upload_size
        .unwrap_or(400 * 1024 * 1024);

    Router::new()
        // HTML pages (serve static files)
        .route("/", get(handlers::serve_index))
        .route("/search", get(handlers::serve_search))
        .route("/upload", get(handlers::serve_upload))
        .route("/changelog", get(handlers::serve_changelog))
        .route("/paste/:id", get(handlers::serve_paste))
        .route("/status", get(handlers::serve_status))
        .route("/disclaimer", get(handlers::serve_disclaimer))
        .route("/raw/:id", get(handlers::raw_paste))
        // Static assets
        .nest_service("/static", ServeDir::new("static"))
        // API endpoints
        .route("/api/pastes", get(handlers::get_pastes))
        .route("/api/search", get(handlers::search_api))
        .route("/api/stats", get(handlers::statistics))
        // HTMX HTML partial endpoints
        .route("/api/pastes/html", get(handlers::get_pastes_html))
        .route("/api/stats/html", get(handlers::get_stats_html))
        .route("/api/search/html", get(handlers::get_search_html))
        .route("/api/search/suggestions", get(handlers::get_search_suggestions))
        .route("/api/health", get(health_check))
        .route("/api/scrapers/health", get(handlers::scraper_health))
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
        // Deduplication check for telegram scraper
        .route("/api/check-hash/:hash", get(handlers::check_hash))
        // Forward telegram invites to scraper
        .route("/api/x/telegram-invite", post(handlers::forward_telegram_invite))
        // Admin API (hidden, token-protected)
        .route("/api/x/login", post(handlers::admin_login))
        .route("/api/x/logout", post(handlers::admin_logout))
        .route("/api/x/stats", get(handlers::admin_stats))
        .route("/api/x/pastes", get(handlers::admin_list_pastes))
        .route("/api/x/paste/:id", delete(handlers::admin_delete_paste))
        .route("/api/x/comment/:id", delete(handlers::admin_delete_comment))
        .route("/api/x/source/:name", delete(handlers::admin_purge_source))
        .route("/api/x/staff-post", post(handlers::admin_create_staff_post))
        // Admin analytics endpoints
        .route("/api/x/analytics", get(handlers::admin_analytics))
        .route("/api/x/activity", get(handlers::admin_activity_logs))
        .route("/api/x/activity/counts", get(handlers::admin_activity_counts))
        // Bulk delete endpoints
        .route("/api/x/sources", get(handlers::admin_list_sources))
        .route("/api/x/bulk-delete", post(handlers::admin_bulk_delete))
        .route("/api/x/all", delete(handlers::admin_delete_all))
        .route("/api/x/older-than/:days", delete(handlers::admin_delete_older_than))
        .route("/api/x/delete-by-search", post(handlers::admin_delete_by_search))
        // Admin panel page (hidden)
        .route("/x", get(handlers::serve_admin))
        .layer(DefaultBodyLimit::max(upload_limit)) // Configurable upload limit
        .layer(CompressionLayer::new())
        // Security headers - prevent tracking, XSS, clickjacking
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff")
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY")
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_XSS_PROTECTION,
            HeaderValue::from_static("1; mode=block")
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer")
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            // Allow HTMX and Alpine.js from unpkg CDN
            HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://unpkg.com; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'")
        ))
        .layer(SetResponseHeaderLayer::overriding(
            "Permissions-Policy".parse::<axum::http::header::HeaderName>().unwrap(),
            HeaderValue::from_static("geolocation=(), microphone=(), camera=(), interest-cohort=()")
        ))
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
