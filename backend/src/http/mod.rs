mod auth;
mod error;
mod export;
mod health;
mod images;
mod questions;
mod render_jobs;

use axum::{
    Router,
    http::StatusCode,
    middleware,
    routing::{MethodRouter, get},
};
use sqlx::PgPool;

use crate::{images::ImageStore, render_jobs::Artifacts};

use self::error::ApiError;

type AdminRoute = (&'static str, MethodRouter<AppState>);

#[derive(Clone)]
struct AppState {
    database: Option<PgPool>,
    images: ImageStore,
    render: Option<Artifacts>,
}

impl AppState {
    fn pool(&self) -> Result<&PgPool, ApiError> {
        self.database.as_ref().ok_or_else(|| {
            ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "database_not_configured")
        })
    }
}

pub(crate) fn router(
    database: Option<PgPool>,
    images: ImageStore,
    render: Option<Artifacts>,
) -> Router {
    let admin = questions::admin_routes()
        .into_iter()
        .chain(images::admin_routes())
        .fold(Router::new(), |router, (path, methods)| {
            router.route(
                path,
                methods.route_layer(middleware::from_fn(auth::require_admin)),
            )
        });

    Router::new()
        .route("/api/health", get(health::check))
        .merge(questions::public_routes())
        .merge(images::public_routes())
        .route("/api/questions/{id}/export", get(export::download))
        .merge(render_jobs::routes())
        .merge(admin)
        .with_state(AppState {
            database,
            images,
            render,
        })
}
