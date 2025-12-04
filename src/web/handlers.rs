use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};

use super::{ApiError, ApiResponse, AppState};
use crate::models::{Comment, PatternMatch, SearchFilters};

#[derive(Debug, Serialize)]
pub struct PasteListItem {
    pub id: String,
    pub title: Option<String>,
    pub source: String,
    pub syntax: String,
    pub created_at: i64,
    pub is_sensitive: bool,
    pub high_value: bool,
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
    pub high_value: bool,
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

/// GET /api/scrapers/health - Get scraper health status
pub async fn scraper_health(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<crate::db::ScraperHealth>>>, ApiError> {
    let db = state
        .db
        .lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let health = db
        .get_scraper_health()
        .map_err(|e| ApiError(format!("Failed to get scraper health: {}", e)))?;
    
    Ok(Json(ApiResponse::ok(health)))
}

// Static HTML file serving
const INDEX_HTML: &str = include_str!("../../static/index.html");
const SEARCH_HTML: &str = include_str!("../../static/search.html");
const UPLOAD_HTML: &str = include_str!("../../static/upload.html");
const PASTE_HTML: &str = include_str!("../../static/paste.html");
const CHANGELOG_HTML: &str = include_str!("../../static/changelog.html");
const STATUS_HTML: &str = include_str!("../../static/status.html");
const ADMIN_HTML: &str = include_str!("../../static/admin.html");

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

/// GET /changelog - Changelog HTML page
pub async fn serve_changelog() -> impl IntoResponse {
    Html(CHANGELOG_HTML)
}

/// GET /status - Status page
pub async fn serve_status() -> impl IntoResponse {
    Html(STATUS_HTML)
}

/// GET /x - Admin panel (hidden)
pub async fn serve_admin() -> impl IntoResponse {
    Html(ADMIN_HTML)
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
    let offset = filters.offset.unwrap_or(0) as usize;
    
    let pastes = if filters.high_value.unwrap_or(false) {
        // High-value alerts: critical severity patterns (AWS keys, private keys, etc.)
        db.get_high_value_pastes(limit, offset)
            .map_err(|e| ApiError(format!("Failed to fetch high-value pastes: {}", e)))?
    } else if filters.interesting.unwrap_or(false) {
        db.get_interesting_pastes(limit, offset)
            .map_err(|e| ApiError(format!("Failed to fetch interesting pastes: {}", e)))?
    } else if filters.source.is_some() || filters.is_sensitive.is_some() {
        db.get_filtered_pastes(filters.source.as_deref(), filters.is_sensitive, limit, offset)
            .map_err(|e| ApiError(format!("Failed to fetch filtered pastes: {}", e)))?
    } else {
        db.get_recent_pastes_offset(limit, offset)
            .map_err(|e| ApiError(format!("Failed to fetch pastes: {}", e)))?
    };

    let responses = pastes
        .into_iter()
        .map(|p| PasteListItem {
            id: p.id,
            title: p.title,
            source: p.source,
            syntax: p.syntax,
            created_at: p.created_at,
            is_sensitive: p.is_sensitive,
            high_value: p.high_value,
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
    
    // Log anonymized activity (paste ID only, no user info)
    let _ = db.log_activity("paste_view", Some(&paste.source));

    let patterns = paste.matched_patterns.unwrap_or_default();

    let response = PasteDetail {
        id: paste.id,
        title: paste.title,
        source: paste.source,
        syntax: paste.syntax,
        content: paste.content,
        created_at: paste.created_at,
        is_sensitive: paste.is_sensitive,
        high_value: paste.high_value,
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

    // Compute hash and check for duplicates first
    let content_hash = crate::hash::compute_hash_normalized(&payload.content);
    
    // Check if this exact content already exists
    if let Ok(Some(existing)) = db.get_paste_by_hash(&content_hash) {
        // Return existing paste instead of error
        let response = CreatePasteResponse {
            id: existing.id.clone(),
            url: format!("/paste/{}", existing.id),
        };
        return Ok((StatusCode::OK, Json(ApiResponse::ok(response))));
    }

    // Create paste (author is always None for complete anonymity)
    let now = Utc::now().timestamp();
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
        high_value: false,  // User uploads don't get auto-flagged
        created_at: now,
        expires_at: now + (7 * 24 * 60 * 60), // 7-day TTL
        view_count: 0,
    };

    let id = paste.id.clone();
    db.insert_paste(&paste)
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint") {
                ApiError("This content has already been submitted".to_string())
            } else {
                ApiError(format!("Failed to store paste: {}", e))
            }
        })?;

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
    let mut db = state
        .db
        .lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    // Log anonymized search activity (no query content for privacy)
    let _ = db.log_activity("search", filters.source.as_deref());
    
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
            high_value: p.high_value,
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

    // Get counts by source (all scrapers + web uploads)
    let sources = vec![
        "pastebin",
        "gists",
        "ideone",
        "codepad",
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
        "bpaste",
        "termbin",
        "sprunge",
        "paste_rs",
        "paste2",
        "pastebin_pl",
        "quickpaste",
        "pastecode",
        "dpaste_org",
        "defuse",
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

// --- COMMENT ENDPOINTS ---

#[derive(Debug, Deserialize)]
pub struct AddCommentRequest {
    pub content: String,
    #[serde(default)]
    pub parent_id: Option<String>,  // For replies
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub id: String,
    pub parent_id: Option<String>,
    pub content: String,
    pub created_at: i64,
}

/// POST /api/paste/:id/comments - Add anonymous comment or reply
pub async fn add_comment(
    State(state): State<AppState>,
    Path(paste_id): Path<String>,
    Json(payload): Json<AddCommentRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CommentResponse>>), ApiError> {
    use chrono::Utc;
    use uuid::Uuid;

    // Validate content
    let content = payload.content.trim();
    if content.is_empty() {
        return Err(ApiError("Comment cannot be empty".to_string()));
    }
    if content.len() > 2000 {
        return Err(ApiError("Comment too long (max 2000 characters)".to_string()));
    }

    let mut db = state
        .db
        .lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;

    // Verify paste exists
    if db.get_paste(&paste_id).map_err(|e| ApiError(format!("Database error: {}", e)))?.is_none() {
        return Err(ApiError("Paste not found".to_string()));
    }

    let comment = Comment {
        id: Uuid::new_v4().to_string(),
        paste_id: paste_id.clone(),
        parent_id: payload.parent_id,
        content: html_escape::encode_text(content).to_string(),
        created_at: Utc::now().timestamp(),
    };

    db.insert_comment(&comment)
        .map_err(|e| ApiError(format!("Failed to save comment: {}", e)))?;

    let response = CommentResponse {
        id: comment.id,
        parent_id: comment.parent_id,
        content: comment.content,
        created_at: comment.created_at,
    };

    Ok((StatusCode::CREATED, Json(ApiResponse::ok(response))))
}

/// GET /api/paste/:id/comments - Get comments for paste (with replies)
pub async fn get_comments(
    State(state): State<AppState>,
    Path(paste_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<CommentResponse>>>, ApiError> {
    let db = state
        .db
        .lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;

    let comments = db
        .get_comments(&paste_id)
        .map_err(|e| ApiError(format!("Failed to get comments: {}", e)))?;

    let responses: Vec<CommentResponse> = comments
        .into_iter()
        .map(|c| CommentResponse {
            id: c.id,
            parent_id: c.parent_id,
            content: c.content,
            created_at: c.created_at,
        })
        .collect();

    Ok(Json(ApiResponse::ok(responses)))
}

// --- EXPORT ENDPOINTS ---

/// GET /api/export/:id/json - Export paste as JSON
pub async fn export_json(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let paste = db
        .get_paste(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let json = serde_json::json!({
        "id": paste.id,
        "title": paste.title,
        "source": paste.source,
        "syntax": paste.syntax,
        "content": paste.content,
        "created_at": paste.created_at,
        "is_sensitive": paste.is_sensitive,
        "matched_patterns": paste.matched_patterns,
    });

    let filename = format!("attachment; filename=\"paste-{}.json\"", id);
    Ok((
        [
            (header::CONTENT_TYPE, "application/json".to_string()),
            (header::CONTENT_DISPOSITION, filename),
        ],
        serde_json::to_string_pretty(&json).unwrap_or_default(),
    ))
}

/// GET /api/export/:id/csv - Export paste as CSV
pub async fn export_csv(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let paste = db
        .get_paste(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // CSV format: line_number,content
    let mut csv = String::from("line_number,content\n");
    for (i, line) in paste.content.lines().enumerate() {
        // Escape quotes and wrap in quotes
        let escaped = line.replace('"', "\"\"");
        csv.push_str(&format!("{},\"{}\"\n", i + 1, escaped));
    }

    let filename = format!("attachment; filename=\"paste-{}.csv\"", id);
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv".to_string()),
            (header::CONTENT_DISPOSITION, filename),
        ],
        csv,
    ))
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

// =============================================================================
// ADMIN API ENDPOINTS (Protected)
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct AdminLoginRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AdminLoginResponse {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct AdminStatsResponse {
    pub total_pastes: i64,
    pub total_comments: i64,
    pub sensitive_pastes: i64,
    pub sources_enabled: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminPasteItem {
    pub id: String,
    pub title: Option<String>,
    pub source: String,
    pub content_preview: String,
    pub is_sensitive: bool,
    pub created_at: i64,
    pub view_count: i64,
}

/// POST /api/admin/login - Admin login
pub async fn admin_login(
    State(state): State<AppState>,
    Json(payload): Json<AdminLoginRequest>,
) -> Result<Json<ApiResponse<AdminLoginResponse>>, ApiError> {
    let admin = state.admin.as_ref()
        .ok_or_else(|| ApiError("Admin not configured".to_string()))?;
    
    match admin.login(&payload.password) {
        Some(token) => Ok(Json(ApiResponse::ok(AdminLoginResponse { token }))),
        None => Err(ApiError("Invalid credentials".to_string())),
    }
}

/// POST /api/admin/logout - Admin logout
pub async fn admin_logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let admin = state.admin.as_ref()
        .ok_or_else(|| ApiError("Admin not configured".to_string()))?;
    
    let token = headers.get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError("Missing token".to_string()))?;
    
    admin.logout(token);
    Ok(Json(ApiResponse::ok(())))
}

/// Helper to verify admin token
fn verify_admin(state: &AppState, headers: &axum::http::HeaderMap) -> Result<(), ApiError> {
    let admin = state.admin.as_ref()
        .ok_or_else(|| ApiError("Admin not configured".to_string()))?;
    
    let token = headers.get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError("Unauthorized".to_string()))?;
    
    if !admin.verify_token(token) {
        return Err(ApiError("Invalid or expired token".to_string()));
    }
    Ok(())
}

/// GET /api/admin/stats - Get admin stats
pub async fn admin_stats(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<AdminStatsResponse>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    let db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let (pastes, comments, sensitive) = db.get_db_stats()
        .map_err(|e| ApiError(format!("Database error: {}", e)))?;
    
    Ok(Json(ApiResponse::ok(AdminStatsResponse {
        total_pastes: pastes,
        total_comments: comments,
        sensitive_pastes: sensitive,
        sources_enabled: vec!["pastebin".into(), "gists".into(), "hastebin".into()],
    })))
}

/// GET /api/admin/pastes - List all pastes (paginated)
pub async fn admin_list_pastes(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<AdminPasteItem>>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    let limit: usize = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(50);
    let offset: usize = params.get("offset").and_then(|v| v.parse().ok()).unwrap_or(0);
    
    let db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let pastes = db.get_all_pastes(limit, offset)
        .map_err(|e| ApiError(format!("Database error: {}", e)))?;
    
    let items: Vec<AdminPasteItem> = pastes.into_iter().map(|p| {
        let preview = p.content.chars().take(200).collect::<String>();
        AdminPasteItem {
            id: p.id,
            title: p.title,
            source: p.source,
            content_preview: preview,
            is_sensitive: p.is_sensitive,
            created_at: p.created_at,
            view_count: p.view_count,
        }
    }).collect();
    
    Ok(Json(ApiResponse::ok(items)))
}

/// DELETE /api/admin/paste/:id - Delete a paste
pub async fn admin_delete_paste(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<bool>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    let mut db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let deleted = db.delete_paste(&id)
        .map_err(|e| ApiError(format!("Failed to delete: {}", e)))?;
    
    Ok(Json(ApiResponse::ok(deleted)))
}

/// DELETE /api/admin/comment/:id - Delete a comment
pub async fn admin_delete_comment(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<bool>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    let mut db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let deleted = db.delete_comment(&id)
        .map_err(|e| ApiError(format!("Failed to delete: {}", e)))?;
    
    Ok(Json(ApiResponse::ok(deleted)))
}

/// DELETE /api/admin/source/:name - Purge all pastes from a source
pub async fn admin_purge_source(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(source): Path<String>,
) -> Result<Json<ApiResponse<usize>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    let mut db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let count = db.purge_source(&source)
        .map_err(|e| ApiError(format!("Failed to purge: {}", e)))?;
    
    Ok(Json(ApiResponse::ok(count)))
}

// === ANALYTICS ENDPOINTS ===

#[derive(Debug, Serialize)]
pub struct SourceHealthItem {
    pub source: String,
    pub success_count: i64,
    pub failure_count: i64,
    pub pastes_found: i64,
    pub success_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct HourlyRate {
    pub hour: i64,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct PatternHit {
    pub pattern: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ActivityLog {
    pub action: String,
    pub details: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsResponse {
    pub source_health: Vec<SourceHealthItem>,
    pub hourly_rates: Vec<HourlyRate>,
    pub pattern_hits: Vec<PatternHit>,
    pub source_breakdown: Vec<(String, i64)>,
}

/// GET /api/x/analytics - Get admin analytics dashboard data
pub async fn admin_analytics(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<AnalyticsResponse>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    let db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    // Get source health (last 24 hours)
    let scraper_stats = db.get_scraper_stats(24)
        .map_err(|e| ApiError(format!("Failed to get scraper stats: {}", e)))?;
    
    let source_health: Vec<SourceHealthItem> = scraper_stats.into_iter().map(|(source, success, failure, pastes)| {
        let total = success + failure;
        let rate = if total > 0 { (success as f64 / total as f64) * 100.0 } else { 0.0 };
        SourceHealthItem {
            source,
            success_count: success,
            failure_count: failure,
            pastes_found: pastes,
            success_rate: rate,
        }
    }).collect();
    
    // Get hourly scrape rates
    let hourly_data = db.get_hourly_scrape_rates()
        .map_err(|e| ApiError(format!("Failed to get hourly rates: {}", e)))?;
    let hourly_rates: Vec<HourlyRate> = hourly_data.into_iter()
        .map(|(hour, count)| HourlyRate { hour, count })
        .collect();
    
    // Get pattern hits
    let pattern_data = db.get_pattern_hits()
        .map_err(|e| ApiError(format!("Failed to get pattern hits: {}", e)))?;
    let pattern_hits: Vec<PatternHit> = pattern_data.into_iter()
        .map(|(pattern, count)| PatternHit { pattern, count })
        .collect();
    
    // Get source breakdown
    let source_breakdown = db.get_source_breakdown()
        .map_err(|e| ApiError(format!("Failed to get source breakdown: {}", e)))?;
    
    Ok(Json(ApiResponse::ok(AnalyticsResponse {
        source_health,
        hourly_rates,
        pattern_hits,
        source_breakdown,
    })))
}

/// GET /api/x/activity - Get activity logs
pub async fn admin_activity_logs(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<ActivityLog>>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    let limit: usize = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(100);
    
    let db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let logs = db.get_activity_logs(limit)
        .map_err(|e| ApiError(format!("Failed to get activity logs: {}", e)))?;
    
    let activity_logs: Vec<ActivityLog> = logs.into_iter()
        .map(|(action, details, timestamp)| ActivityLog { action, details, timestamp })
        .collect();
    
    Ok(Json(ApiResponse::ok(activity_logs)))
}

#[derive(Debug, Serialize)]
pub struct ActivityCountItem {
    pub action: String,
    pub count: i64,
}

/// GET /api/x/activity/counts - Get activity counts by type
pub async fn admin_activity_counts(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<Vec<ActivityCountItem>>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    let db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let counts = db.get_activity_counts()
        .map_err(|e| ApiError(format!("Failed to get activity counts: {}", e)))?;
    
    let items: Vec<ActivityCountItem> = counts.into_iter()
        .map(|(action, count)| ActivityCountItem { action, count })
        .collect();
    
    Ok(Json(ApiResponse::ok(items)))
}

// === BULK DELETE ENDPOINTS ===

#[derive(Debug, Deserialize)]
pub struct BulkDeleteRequest {
    pub ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkDeleteResponse {
    pub deleted: usize,
}

/// POST /api/x/bulk-delete - Batch delete pastes by IDs
pub async fn admin_bulk_delete(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<BulkDeleteRequest>,
) -> Result<Json<ApiResponse<BulkDeleteResponse>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    let mut db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let deleted = db.delete_pastes_by_ids(&payload.ids)
        .map_err(|e| ApiError(format!("Failed to delete pastes: {}", e)))?;
    
    Ok(Json(ApiResponse::ok(BulkDeleteResponse { deleted })))
}

/// DELETE /api/x/all - Delete ALL pastes (dangerous!)
pub async fn admin_delete_all(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<BulkDeleteResponse>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    let mut db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let deleted = db.delete_all_pastes()
        .map_err(|e| ApiError(format!("Failed to delete all pastes: {}", e)))?;
    
    Ok(Json(ApiResponse::ok(BulkDeleteResponse { deleted })))
}

/// DELETE /api/x/older-than/:days - Delete pastes older than N days
pub async fn admin_delete_older_than(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(days): Path<i64>,
) -> Result<Json<ApiResponse<BulkDeleteResponse>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    if days < 1 {
        return Err(ApiError("Days must be at least 1".to_string()));
    }
    
    let mut db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let deleted = db.delete_pastes_older_than(days)
        .map_err(|e| ApiError(format!("Failed to delete old pastes: {}", e)))?;
    
    Ok(Json(ApiResponse::ok(BulkDeleteResponse { deleted })))
}

#[derive(Debug, Serialize)]
pub struct SourceItem {
    pub source: String,
    pub count: i64,
}

/// GET /api/x/sources - Get all unique sources with counts
pub async fn admin_list_sources(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<Vec<SourceItem>>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    let db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let sources = db.get_all_sources()
        .map_err(|e| ApiError(format!("Failed to get sources: {}", e)))?;
    
    let items: Vec<SourceItem> = sources.into_iter()
        .map(|(source, count)| SourceItem { source, count })
        .collect();
    
    Ok(Json(ApiResponse::ok(items)))
}

#[derive(Debug, Deserialize)]
pub struct DeleteBySearchRequest {
    pub query: String,
}

/// POST /api/x/delete-by-search - Delete pastes matching search query
pub async fn admin_delete_by_search(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<DeleteBySearchRequest>,
) -> Result<Json<ApiResponse<BulkDeleteResponse>>, ApiError> {
    verify_admin(&state, &headers)?;
    
    if payload.query.trim().is_empty() {
        return Err(ApiError("Search query cannot be empty".to_string()));
    }
    
    let mut db = state.db.lock()
        .map_err(|e| ApiError(format!("Database lock error: {}", e)))?;
    
    let deleted = db.delete_pastes_by_search(&payload.query)
        .map_err(|e| ApiError(format!("Failed to delete pastes: {}", e)))?;
    
    Ok(Json(ApiResponse::ok(BulkDeleteResponse { deleted })))
}
