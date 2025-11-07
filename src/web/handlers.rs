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
    let _db = state.db.lock().unwrap();
    // TODO: Fetch recent pastes from database
    Ok(Json(ApiResponse::ok(vec![])))
}

/// GET /paste/:id - View individual paste
pub async fn view_paste(
    State(state): State<AppState>,
    Path(_id): Path<String>,
) -> Result<Json<ApiResponse<PasteResponse>>, ApiError> {
    let _db = state.db.lock().unwrap();
    // TODO: Fetch paste by ID from database
    Err(ApiError("Not found".to_string()))
}

/// GET /raw/:id - Raw text view
pub async fn raw_paste(
    State(state): State<AppState>,
    Path(_id): Path<String>,
) -> Result<String, StatusCode> {
    let _db = state.db.lock().unwrap();
    // TODO: Fetch paste raw content from database
    Err(StatusCode::NOT_FOUND)
}

/// POST /upload - Submit new paste
pub async fn upload_paste(
    State(state): State<AppState>,
    Json(_payload): Json<UploadRequest>,
) -> Result<(StatusCode, Json<ApiResponse<String>>), ApiError> {
    let _db = state.db.lock().unwrap();
    // TODO: Validate and store new paste in database
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::ok("paste_id".to_string())),
    ))
}

/// GET /search - Full-text search
pub async fn search(
    State(state): State<AppState>,
    Query(_filters): Query<SearchFilters>,
) -> Result<Json<ApiResponse<Vec<PasteResponse>>>, ApiError> {
    let _db = state.db.lock().unwrap();
    // TODO: Perform FTS search with filters from database
    Ok(Json(ApiResponse::ok(vec![])))
}
