use axum::{Json, extract::State};
use serde::Serialize;

use super::AppState;
use crate::db;

#[derive(Serialize)]
pub(super) struct HealthResponse {
    api: &'static str,
    database: DatabaseStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum DatabaseStatus {
    NotConfigured,
    Connected,
    Unavailable,
}

pub(super) async fn check(State(state): State<AppState>) -> Json<HealthResponse> {
    let database = match state.database {
        None => DatabaseStatus::NotConfigured,
        Some(pool) if db::ping(&pool).await => DatabaseStatus::Connected,
        Some(_) => DatabaseStatus::Unavailable,
    };

    Json(HealthResponse {
        api: "ok",
        database,
    })
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use sqlx::PgPool;
    use tower::ServiceExt;

    use crate::{db, http, images::ImageStore, storage::LocalFileStore};

    #[tokio::test]
    async fn reports_unconfigured_database() {
        assert_health(None, "not_configured").await;
    }

    #[tokio::test]
    async fn reports_unavailable_database_without_marking_api_down() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("postgres://test@{}/test", listener.local_addr().unwrap());
        assert_health(Some(db::connection_pool(&url, 1).unwrap()), "unavailable").await;
    }

    async fn assert_health(database: Option<PgPool>, expected: &str) {
        let directory = tempfile::tempdir().unwrap();
        let response = http::router(
            database,
            ImageStore::new(LocalFileStore::new(directory.path().into())),
            None,
        )
        .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(
            body.as_ref(),
            format!(r#"{{"api":"ok","database":"{expected}"}}"#).as_bytes()
        );
    }
}
