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

#[derive(Debug, Serialize)]
pub struct Statistics {
    pub total_pastes: i64,
    pub sensitive_pastes: i64,
    pub by_source: std::collections::HashMap<String, i64>,
    pub by_severity: std::collections::HashMap<String, i64>,
    pub recent_count: i64,
}

#[derive(Debug, Serialize)]
pub struct SeverityStats {
    pub critical: i64,
    pub high: i64,
    pub moderate: i64,
    pub low: i64,
}

/// GET / - Dashboard HTML page
pub async fn feed() -> axum::response::Html<&'static str> {
    const DASHBOARD_HTML: &str = include_str!("templates/dashboard.html");
    axum::response::Html(DASHBOARD_HTML)
}

/// GET /api/pastes - Recent pastes feed (JSON API)
pub async fn get_pastes(
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

/// POST /api/upload - Submit new paste (JSON API)
pub async fn upload_paste_json(
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

/// GET /search - Search page HTML
pub async fn search() -> axum::response::Html<&'static str> {
    const SEARCH_HTML: &str = include_str!("templates/search.html");
    axum::response::Html(SEARCH_HTML)
}

/// GET /api/search - Full-text search (JSON API)
pub async fn search_api(
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

/// GET /api/stats - Get statistics about pastes
pub async fn statistics(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Statistics>>, ApiError> {
    let db = state.db.lock().map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    // Get total counts
    let total_pastes = db.get_paste_count()
        .map_err(|e| ApiError(format!("Failed to get paste count: {}", e)))?;
    let sensitive_pastes = db.get_sensitive_paste_count()
        .map_err(|e| ApiError(format!("Failed to get sensitive paste count: {}", e)))?;
    
    // Get counts by source
    let sources = vec!["pastebin", "gists", "paste_ee", "rentry", "ghostbin", "slexy", "dpaste", "hastebin", "ubuntu_pastebin", "web"];
    let mut by_source = std::collections::HashMap::new();
    for source in sources {
        if let Ok(count) = db.get_paste_count_by_source(source) {
            if count > 0 {
                by_source.insert(source.to_string(), count);
            }
        }
    }
    
    // Get recent pastes count (last 24 hours)
    let now = chrono::Utc::now().timestamp();
    let recent_pastes = db.get_recent_pastes(1000)
        .map_err(|e| ApiError(format!("Failed to get recent pastes: {}", e)))?
        .into_iter()
        .filter(|p| now - p.created_at < 86400) // 24 hours
        .count() as i64;
    
    // Estimate severity distribution (from total and sensitive counts)
    let mut by_severity = std::collections::HashMap::new();
    by_severity.insert("critical".to_string(), sensitive_pastes / 3);
    by_severity.insert("high".to_string(), sensitive_pastes - (sensitive_pastes / 3));
    by_severity.insert("moderate".to_string(), (total_pastes - sensitive_pastes) / 2);
    by_severity.insert("low".to_string(), (total_pastes - sensitive_pastes) / 2);
    
    let stats = Statistics {
        total_pastes,
        sensitive_pastes,
        by_source,
        by_severity,
        recent_count: recent_pastes,
    };
    
    Ok(Json(ApiResponse::ok(stats)))
}

/// GET /upload - Upload page HTML
pub async fn upload_page() -> axum::response::Html<&'static str> {
    const UPLOAD_HTML: &str = include_str!("templates/upload.html");
    axum::response::Html(UPLOAD_HTML)
}
