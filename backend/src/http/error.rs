use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug)]
pub(super) struct ApiError {
    status: StatusCode,
    code: &'static str,
}

impl ApiError {
    pub(super) fn new(status: StatusCode, code: &'static str) -> Self {
        Self { status, code }
    }

    pub(super) fn image_storage() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "image_storage_error")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Body {
            error: &'static str,
        }

        (self.status, Json(Body { error: self.code })).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::Io(_) => {
                Self::new(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable")
            }
            _ => Self::new(StatusCode::INTERNAL_SERVER_ERROR, "database_error"),
        }
    }
}
