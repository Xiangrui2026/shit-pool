use std::io;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};

use super::{AppState, error::ApiError};
use crate::{
    questions,
    render_jobs::{self, Artifact, JobStatus, Snapshot, Submission},
};

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/render-jobs",
            post(submit).layer(DefaultBodyLimit::max(4096)),
        )
        .route("/api/render-jobs/{id}", get(status))
        .route("/api/render-jobs/{id}/files/{kind}", get(download))
}

async fn submit(
    State(state): State<AppState>,
    Json(submission): Json<Submission>,
) -> Result<impl IntoResponse, ApiError> {
    let submission = submission
        .validated()
        .map_err(|code| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, code))?;
    if state.render.is_none() {
        return Err(disabled());
    }
    let pool = state.pool()?;
    let questions = questions::render_selection(pool, &submission.question_ids).await?;
    if questions.len() != submission.question_ids.len() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "paper_question_unavailable",
        ));
    }
    let snapshot = Snapshot {
        title: submission.title,
        template: submission.template,
        questions,
    };
    let id = render_jobs::new_id().map_err(|_| storage_error())?;
    let job = render_jobs::enqueue(pool, &id, snapshot)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::TOO_MANY_REQUESTS, "render_queue_full"))?;
    Ok((
        StatusCode::ACCEPTED,
        [(header::CACHE_CONTROL, "no-store")],
        Json(job),
    ))
}

async fn status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(find(&state, &id).await?),
    ))
}

async fn download(
    State(state): State<AppState>,
    Path((id, kind)): Path<(String, Artifact)>,
) -> Result<Response, ApiError> {
    let job = find(&state, &id).await?;
    if job.status != "completed" || matches!(kind, Artifact::Answers) && !job.has_answers {
        return Err(not_found());
    }
    let store = state.render.ok_or_else(disabled)?;
    let bytes = tokio::task::spawn_blocking(move || store.read(&id, kind))
        .await
        .map_err(|_| storage_error())?
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => not_found(),
            _ => storage_error(),
        })?;
    let name = format!("attachment; filename=\"paper-{}.pdf\"", kind.name());
    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (header::CONTENT_DISPOSITION, name.as_str()),
            (header::CACHE_CONTROL, "no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        bytes,
    )
        .into_response())
}

async fn find(state: &AppState, id: &str) -> Result<JobStatus, ApiError> {
    if !render_jobs::valid_id(id) {
        return Err(not_found());
    }
    if state.render.is_none() {
        return Err(disabled());
    }
    render_jobs::status(state.pool()?, id)
        .await?
        .ok_or_else(not_found)
}

fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "render_job_not_found")
}

fn disabled() -> ApiError {
    ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "render_disabled")
}

fn storage_error() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "render_storage_error")
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    use super::*;
    use crate::{http::router, images::ImageStore, storage::LocalFileStore};

    #[tokio::test]
    async fn public_submission_is_validated_without_admin_key() {
        let dir = tempfile::tempdir().unwrap();
        let app = router(
            None,
            ImageStore::new(LocalFileStore::new(dir.path().into())),
            None,
        );
        for (body, status, reason) in [
            (
                r#"{"title":"卷子","question_ids":[1]}"#,
                StatusCode::SERVICE_UNAVAILABLE,
                "render_disabled",
            ),
            (
                r#"{"title":"卷子","question_ids":[1,1]}"#,
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_paper_questions",
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::post("/api/render-jobs")
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            let body = to_bytes(response.into_body(), 1024).await.unwrap();
            assert_eq!(
                body.as_ref(),
                format!(r#"{{"error":"{reason}"}}"#).as_bytes()
            );
        }
        let response = app
            .oneshot(
                Request::get("/api/render-jobs/not-an-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
