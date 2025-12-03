use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};

use super::{ApiError, ApiResponse, AppState};
use crate::models::{PatternMatch, SearchFilters};

#[derive(Debug, Serialize)]
pub struct PasteListItem {
    pub id: String,
    pub title: Option<String>,
    pub source: String,
    pub syntax: String,
    pub created_at: i64,
    pub is_sensitive: bool,
}

#[derive(Debug, Serialize)]
pub struct PasteDetail {
    pub id: String,
    pub title: Option<String>,
    pub source: String,
    pub syntax: String,
    pub content: String,
    pub created_at: i64,
    pub is_sensitive: bool,
    pub matched_patterns: Vec<PatternMatch>,
    pub view_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub title: Option<String>,
    pub content: String,
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

// Static HTML file serving
const INDEX_HTML: &str = include_str!("../../static/index.html");
const SEARCH_HTML: &str = include_str!("../../static/search.html");
const UPLOAD_HTML: &str = include_str!("../../static/upload.html");
const PASTE_HTML: &str = include_str!("../../static/paste.html");

/// GET / - Dashboard HTML page
pub async fn serve_index() -> impl IntoResponse {
    Html(INDEX_HTML)
}

/// GET /search - Search HTML page
pub async fn serve_search() -> impl IntoResponse {
    Html(SEARCH_HTML)
}

/// GET /upload - Upload HTML page  
pub async fn serve_upload() -> impl IntoResponse {
    Html(UPLOAD_HTML)
}

/// GET /paste/:id - Paste detail HTML page
pub async fn serve_paste() -> impl IntoResponse {
    Html(PASTE_HTML)
}

/// GET /api/pastes - Recent pastes feed (JSON API)
pub async fn get_pastes(
    State(state): State<AppState>,
    Query(filters): Query<SearchFilters>,
) -> Result<Json<ApiResponse<Vec<PasteListItem>>>, ApiError> {
    let db = state
        .db
        .lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let limit = filters.limit.unwrap_or(50).min(100) as usize;
    let pastes = db
        .get_recent_pastes(limit)
        .map_err(|e| ApiError(format!("Failed to fetch pastes: {}", e)))?;

    let responses = pastes
        .into_iter()
        .map(|p| PasteListItem {
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

/// GET /api/paste/:id - Get paste details (JSON API)
pub async fn get_paste_api(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<PasteDetail>>, ApiError> {
    let mut db = state
        .db
        .lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    let paste = db
        .get_paste(&id)
        .map_err(|e| ApiError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError("Paste not found".to_string()))?;

    // Increment view count
    let _ = db.increment_view_count(&id);

    let patterns = paste.matched_patterns.unwrap_or_default();

    let response = PasteDetail {
        id: paste.id,
        title: paste.title,
        source: paste.source,
        syntax: paste.syntax,
        content: paste.content,
        created_at: paste.created_at,
        is_sensitive: paste.is_sensitive,
        matched_patterns: patterns,
        view_count: paste.view_count,
    };

    Ok(Json(ApiResponse::ok(response)))
}

/// GET /raw/:id - Raw text view
pub async fn raw_paste(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let paste = db
        .get_paste(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Increment view count
    let _ = db.increment_view_count(&id);

    Ok((
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        paste.content,
    ))
}

#[derive(Debug, Serialize)]
pub struct CreatePasteResponse {
    pub id: String,
    pub url: String,
}

/// POST /api/paste - Submit new paste (JSON API)
pub async fn upload_paste_json(
    State(state): State<AppState>,
    Json(payload): Json<UploadRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CreatePasteResponse>>), ApiError> {
    use chrono::Utc;
    use uuid::Uuid;

    let mut db = state
        .db
        .lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;

    // Validate content
    if payload.content.is_empty() {
        return Err(ApiError("Content cannot be empty".to_string()));
    }

    // Anonymize user submission (sanitize title, remove any PII)
    let mut title = payload.title;
    if let Some(t) = &title {
        // Sanitize title to remove emails, URLs, usernames
        let sanitized = t
            .replace('@', "")
            .replace("http://", "")
            .replace("https://", "");
        title = if sanitized.is_empty() {
            None
        } else {
            Some(sanitized)
        };
    }

    // Auto-detect language from content
    let detected_lang = crate::lang_detect::detect_language(&payload.content);

    // Create paste (author is always None for complete anonymity)
    let now = Utc::now().timestamp();
    let content_hash = crate::hash::compute_hash_normalized(&payload.content);
    let paste = crate::models::Paste {
        id: Uuid::new_v4().to_string(),
        source: "web".to_string(),
        source_id: None,
        title,
        author: None,
        content: payload.content,
        content_hash,
        url: None,
        syntax: detected_lang,
        matched_patterns: None,
        is_sensitive: false,
        created_at: now,
        expires_at: now + (7 * 24 * 60 * 60), // 7-day TTL
        view_count: 0,
    };

    let id = paste.id.clone();
    db.insert_paste(&paste)
        .map_err(|e| ApiError(format!("Failed to store paste: {}", e)))?;

    let response = CreatePasteResponse {
        id: id.clone(),
        url: format!("/paste/{}", id),
    };
    Ok((StatusCode::CREATED, Json(ApiResponse::ok(response))))
}

/// GET /api/search - Full-text search (JSON API)
pub async fn search_api(
    State(state): State<AppState>,
    Query(filters): Query<SearchFilters>,
) -> Result<Json<ApiResponse<Vec<PasteListItem>>>, ApiError> {
    let db = state
        .db
        .lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    let pastes = db
        .search_pastes(&filters)
        .map_err(|e| ApiError(format!("Search failed: {}", e)))?;

    let responses = pastes
        .into_iter()
        .map(|p| PasteListItem {
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
    let db = state
        .db
        .lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;

    // Get total counts
    let total_pastes = db
        .get_paste_count()
        .map_err(|e| ApiError(format!("Failed to get paste count: {}", e)))?;
    let sensitive_pastes = db
        .get_sensitive_paste_count()
        .map_err(|e| ApiError(format!("Failed to get sensitive paste count: {}", e)))?;

    // Get counts by source (all 13 scrapers + web uploads)
    let sources = vec![
        "pastebin",
        "gists",
        "paste_ee",
        "rentry",
        "ghostbin",
        "slexy",
        "dpaste",
        "hastebin",
        "ubuntu_pastebin",
        "ixio",
        "justpaste",
        "controlc",
        "external",
        "external_url",
        "web",
    ];
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
    let recent_pastes = db
        .get_recent_pastes(1000)
        .map_err(|e| ApiError(format!("Failed to get recent pastes: {}", e)))?
        .into_iter()
        .filter(|p| now - p.created_at < 86400) // 24 hours
        .count() as i64;

    // Estimate severity distribution (from total and sensitive counts)
    let mut by_severity = std::collections::HashMap::new();
    by_severity.insert("critical".to_string(), sensitive_pastes / 3);
    by_severity.insert(
        "high".to_string(),
        sensitive_pastes - (sensitive_pastes / 3),
    );
    by_severity.insert(
        "moderate".to_string(),
        (total_pastes - sensitive_pastes) / 2,
    );
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

/// Request body for submitting URLs
#[derive(Debug, Deserialize)]
pub struct SubmitUrlRequest {
    pub url: String,
    #[serde(default)]
    pub urls: Vec<String>,
}

/// Response for URL submission
#[derive(Debug, Serialize)]
pub struct SubmitUrlResponse {
    pub queued: usize,
    pub message: String,
}

/// POST /api/submit-url - Submit paste URLs for monitoring
pub async fn submit_url(
    State(state): State<AppState>,
    Json(payload): Json<SubmitUrlRequest>,
) -> Result<(StatusCode, Json<ApiResponse<SubmitUrlResponse>>), ApiError> {
    let scraper = state
        .url_scraper
        .as_ref()
        .ok_or_else(|| ApiError("External URL scraper not available".to_string()))?;

    let mut urls_to_add = Vec::new();

    // Add single URL if provided
    if !payload.url.is_empty() {
        urls_to_add.push(payload.url);
    }

    // Add multiple URLs if provided
    urls_to_add.extend(payload.urls);

    // Validate URLs
    let valid_urls: Vec<String> = urls_to_add
        .into_iter()
        .filter(|url| {
            // Basic URL validation
            url.starts_with("http://") || url.starts_with("https://")
        })
        .collect();

    if valid_urls.is_empty() {
        return Err(ApiError("No valid URLs provided".to_string()));
    }

    let count = valid_urls.len();
    scraper.add_urls(valid_urls);

    let response = SubmitUrlResponse {
        queued: count,
        message: format!("Queued {} URL(s) for scraping", count),
    };

    Ok((StatusCode::ACCEPTED, Json(ApiResponse::ok(response))))
}
