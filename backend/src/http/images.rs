use std::io;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get as route_get, post},
};
use serde::Serialize;

use super::{AdminRoute, AppState, error::ApiError};
use crate::images::{ImageError, MAX_IMAGE_SIZE, mime_type};

pub(super) fn public_routes() -> Router<AppState> {
    Router::new().route("/api/images/{name}", route_get(get))
}

pub(super) fn admin_routes() -> [AdminRoute; 1] {
    [(
        "/api/images",
        post(upload).layer(DefaultBodyLimit::max(MAX_IMAGE_SIZE)),
    )]
}

#[derive(Serialize)]
pub(super) struct UploadedImage {
    name: String,
    path: String,
    tex: String,
}

pub(super) async fn upload(
    State(state): State<AppState>,
    data: Bytes,
) -> Result<(StatusCode, Json<UploadedImage>), ApiError> {
    let name = tokio::task::spawn_blocking(move || state.images.save(&data))
        .await
        .map_err(|_| ApiError::image_storage())?
        .map_err(|error| match error {
            ImageError::Unsupported => {
                ApiError::new(StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported_image")
            }
            ImageError::Invalid => ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_image"),
            ImageError::TooLarge => ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "image_too_large"),
            ImageError::Io => ApiError::image_storage(),
        })?;
    let path = format!("images/{name}");
    let tex = format!(r"\includegraphics{{{path}}}");
    Ok((StatusCode::CREATED, Json(UploadedImage { name, path, tex })))
}

pub(super) async fn get(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Response, ApiError> {
    let media = mime_type(&name).ok_or_else(image_not_found)?;
    let data = tokio::task::spawn_blocking(move || state.images.read(&name, MAX_IMAGE_SIZE))
        .await
        .map_err(|_| ApiError::image_storage())?
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => image_not_found(),
            _ => ApiError::image_storage(),
        })?;
    Ok((
        [
            (header::CONTENT_TYPE, media),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        data,
    )
        .into_response())
}

fn image_not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "image_not_found")
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    use super::*;
    use crate::{
        http::{auth, router},
        images::{ImageStore, test_png},
        storage::LocalFileStore,
    };

    #[tokio::test]
    async fn upload_requires_auth_and_download_serves_only_the_stored_image() {
        let directory = tempfile::tempdir().unwrap();
        let app = router(
            None,
            ImageStore::new(LocalFileStore::new(directory.path().into())),
            None,
        );
        let image = test_png();
        let unknown = app
            .clone()
            .oneshot(Request::post("/api/images").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(unknown.status(), StatusCode::UNAUTHORIZED);
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/images")
                    .header(header::AUTHORIZATION, auth::test_authorization())
                    .body(Body::from(image.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let name = directory
            .path()
            .read_dir()
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .file_name();
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/api/images/{}", name.to_string_lossy()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
        assert_eq!(
            to_bytes(response.into_body(), MAX_IMAGE_SIZE)
                .await
                .unwrap()
                .as_ref(),
            image
        );

        let response = app
            .oneshot(
                Request::get("/api/images/..%2Fsecret.png")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
