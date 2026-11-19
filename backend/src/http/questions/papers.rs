use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::{
    http::{AppState, error::ApiError},
    questions::{self, Paper, Question},
};

#[derive(Deserialize)]
pub(super) struct PaperQuery {
    source: String,
}

pub(super) async fn list(State(state): State<AppState>) -> Result<Json<Vec<Paper>>, ApiError> {
    Ok(Json(questions::papers(state.pool()?).await?))
}

pub(super) async fn detail(
    State(state): State<AppState>,
    Query(query): Query<PaperQuery>,
) -> Result<Json<Vec<Question>>, ApiError> {
    let source = query.source.trim();
    if source.is_empty() || source.contains('\0') || source.chars().count() > 300 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "invalid_source"));
    }
    let items = questions::paper_questions(state.pool()?, source).await?;
    if items.is_empty() {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "paper_not_found"));
    }
    Ok(Json(items))
}
