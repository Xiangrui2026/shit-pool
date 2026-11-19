use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;

use super::{not_found, valid_id};
use crate::{
    http::{AppState, error::ApiError},
    questions::{self, NewSolution, SolutionPatch},
};

#[derive(Serialize)]
pub(super) struct SolutionId {
    question_id: i32,
    id: i32,
}

pub(super) async fn create(
    State(state): State<AppState>,
    Path(question_id): Path<i32>,
    Json(solution): Json<NewSolution>,
) -> Result<(StatusCode, Json<SolutionId>), ApiError> {
    let question_id = valid_id(question_id)?;
    let solution = solution.validated().map_err(unprocessable)?;
    let id = questions::add_solution(state.pool()?, question_id, solution)
        .await?
        .ok_or_else(not_found)?;
    Ok((StatusCode::CREATED, Json(SolutionId { question_id, id })))
}

pub(super) async fn update(
    State(state): State<AppState>,
    Path((question_id, id)): Path<(i32, i32)>,
    Json(patch): Json<SolutionPatch>,
) -> Result<Json<SolutionId>, ApiError> {
    let question_id = valid_id(question_id)?;
    let id = valid_id(id)?;
    let patch = patch.validated().map_err(unprocessable)?;
    if !questions::update_solution(state.pool()?, question_id, id, patch).await? {
        return Err(solution_not_found());
    }
    Ok(Json(SolutionId { question_id, id }))
}

pub(super) async fn delete(
    State(state): State<AppState>,
    Path((question_id, id)): Path<(i32, i32)>,
) -> Result<Json<SolutionId>, ApiError> {
    let question_id = valid_id(question_id)?;
    let id = valid_id(id)?;
    if !questions::delete_solution(state.pool()?, question_id, id).await? {
        return Err(solution_not_found());
    }
    Ok(Json(SolutionId { question_id, id }))
}

fn unprocessable(code: &'static str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, code)
}

fn solution_not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "solution_not_found")
}
