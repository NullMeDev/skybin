use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::stats::{SharedStats, StatsResponse};

/// Server state
#[derive(Clone)]
pub struct AppState {
    pub stats: SharedStats,
}

/// Start the HTTP server for stats and invite receiving
pub async fn start_server(stats: SharedStats, port: u16) {
    let state = AppState { stats };
    
    let app = Router::new()
        .route("/health", get(health))
        .route("/stats", get(get_stats))
        .route("/invite", post(receive_invite))
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state));
    
    let addr = format!("0.0.0.0:{}", port);
    info!("📊 Stats server listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Health check endpoint
async fn health() -> &'static str {
    "ok"
}

/// Get scraper statistics
async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Json<StatsApiResponse> {
    let stats = state.stats.to_json();
    Json(StatsApiResponse {
        success: true,
        data: Some(stats),
        error: None,
    })
}

#[derive(Serialize)]
struct StatsApiResponse {
    success: bool,
    data: Option<StatsResponse>,
    error: Option<String>,
}

/// Receive invite link from SkyBin
#[derive(Deserialize)]
struct InviteRequest {
    invite: String,
}

#[derive(Serialize)]
struct InviteResponse {
    success: bool,
    message: String,
}

async fn receive_invite(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InviteRequest>,
) -> (StatusCode, Json<InviteResponse>) {
    // Extract invite hash from various formats
    let invite = extract_invite_hash(&req.invite);
    
    if let Some(hash) = invite {
        info!("📥 Received invite link: {}", hash);
        state.stats.add_pending_invite(hash);
        
        (StatusCode::OK, Json(InviteResponse {
            success: true,
            message: "Invite queued for joining".to_string(),
        }))
    } else {
        (StatusCode::BAD_REQUEST, Json(InviteResponse {
            success: false,
            message: "Invalid invite format".to_string(),
        }))
    }
}

/// Extract invite hash from URL or raw hash
fn extract_invite_hash(input: &str) -> Option<String> {
    let input = input.trim();
    
    // Format: t.me/+HASH or t.me/joinchat/HASH
    if input.contains("t.me/+") {
        if let Some(hash) = input.split("t.me/+").nth(1) {
            let hash = hash.split_whitespace().next().unwrap_or(hash);
            let hash = hash.trim_end_matches(|c| c == '/' || c == '?' || c == '"');
            if !hash.is_empty() {
                return Some(hash.to_string());
            }
        }
    }
    
    if input.contains("t.me/joinchat/") {
        if let Some(hash) = input.split("t.me/joinchat/").nth(1) {
            let hash = hash.split_whitespace().next().unwrap_or(hash);
            let hash = hash.trim_end_matches(|c| c == '/' || c == '?' || c == '"');
            if !hash.is_empty() {
                return Some(hash.to_string());
            }
        }
    }
    
    // Format: @username
    if input.starts_with('@') {
        return Some(input[1..].to_string());
    }
    
    // Raw hash (long alphanumeric)
    if input.len() > 10 && input.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Some(input.to_string());
    }
    
    // Raw username
    if input.chars().all(|c| c.is_alphanumeric() || c == '_') && input.len() >= 5 {
        return Some(input.to_string());
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_invite_hash() {
        assert_eq!(
            extract_invite_hash("https://t.me/+AbCdEfGhIjK"),
            Some("AbCdEfGhIjK".to_string())
        );
        assert_eq!(
            extract_invite_hash("t.me/+AbCdEfGhIjK"),
            Some("AbCdEfGhIjK".to_string())
        );
        assert_eq!(
            extract_invite_hash("https://t.me/joinchat/AbCdEfGhIjK"),
            Some("AbCdEfGhIjK".to_string())
        );
        assert_eq!(
            extract_invite_hash("@channelname"),
            Some("channelname".to_string())
        );
        assert_eq!(
            extract_invite_hash("leaboratory"),
            Some("leaboratory".to_string())
        );
    }
}
