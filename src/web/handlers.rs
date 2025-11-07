use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use super::{ApiError, ApiResponse, AppState};
use crate::models::SearchFilters;

#[derive(Debug, Serialize)]
pub struct PasteResponse {
    pub id: String,
    pub title: Option<String>,
    pub source: String,
    pub syntax: String,
    pub created_at: i64,
    pub is_sensitive: bool,
}

#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub title: Option<String>,
    pub content: String,
    pub syntax: Option<String>,
}

/// GET / - Recent pastes feed
pub async fn feed(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<PasteResponse>>>, ApiError> {
    let db = state.db.lock().map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    let pastes = db.get_recent_pastes(50)
        .map_err(|e| ApiError(format!("Failed to fetch pastes: {}", e)))?;
    
    let responses = pastes.into_iter()
        .map(|p| PasteResponse {
            id: p.id,
            title: p.title,
            source: p.source,
            syntax: p.syntax,
            created_at: p.created_at,
            is_sensitive: p.is_sensitive,
        })
        .collect();
    
    Ok(Json(ApiResponse::ok(responses)))
}

/// GET /paste/:id - View individual paste
pub async fn view_paste(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<PasteResponse>>, ApiError> {
    let mut db = state.db.lock().map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    let paste = db.get_paste(&id)
        .map_err(|e| ApiError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError("Paste not found".to_string()))?;
    
    // Increment view count
    let _ = db.increment_view_count(&id);
    
    let response = PasteResponse {
        id: paste.id,
        title: paste.title,
        source: paste.source,
        syntax: paste.syntax,
        created_at: paste.created_at,
        is_sensitive: paste.is_sensitive,
    };
    
    Ok(Json(ApiResponse::ok(response)))
}

/// GET /raw/:id - Raw text view
pub async fn raw_paste(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<String, StatusCode> {
    let mut db = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let paste = db.get_paste(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Increment view count
    let _ = db.increment_view_count(&id);
    
    Ok(paste.content)
}

/// POST /upload - Submit new paste
pub async fn upload_paste(
    State(state): State<AppState>,
    Json(payload): Json<UploadRequest>,
) -> Result<(StatusCode, Json<ApiResponse<String>>), ApiError> {
    use uuid::Uuid;
    use chrono::Utc;
    
    let mut db = state.db.lock().map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    // Validate content
    if payload.content.is_empty() {
        return Err(ApiError("Content cannot be empty".to_string()));
    }
    
    // Create paste
    let now = Utc::now().timestamp();
    let content_hash = crate::hash::compute_hash_normalized(&payload.content);
    let paste = crate::models::Paste {
        id: Uuid::new_v4().to_string(),
        source: "web".to_string(),
        source_id: None,
        title: payload.title,
        author: None,
        content: payload.content,
        content_hash,
        url: None,
        syntax: payload.syntax.unwrap_or_else(|| "plaintext".to_string()),
        matched_patterns: None,
        is_sensitive: false,
        created_at: now,
        expires_at: now + (7 * 24 * 60 * 60), // 7-day TTL
        view_count: 0,
    };
    
    let id = paste.id.clone();
    db.insert_paste(&paste)
        .map_err(|e| ApiError(format!("Failed to store paste: {}", e)))?;
    
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::ok(id)),
    ))
}

/// GET /search - Full-text search
pub async fn search(
    State(state): State<AppState>,
    Query(filters): Query<SearchFilters>,
) -> Result<Json<ApiResponse<Vec<PasteResponse>>>, ApiError> {
    let db = state.db.lock().map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    let pastes = db.search_pastes(&filters)
        .map_err(|e| ApiError(format!("Search failed: {}", e)))?;
    
    let responses = pastes.into_iter()
        .map(|p| PasteResponse {
            id: p.id,
            title: p.title,
            source: p.source,
            syntax: p.syntax,
            created_at: p.created_at,
            is_sensitive: p.is_sensitive,
        })
        .collect();
    
    Ok(Json(ApiResponse::ok(responses)))
}
