use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;

use super::error::ApiError;

const ADMIN_KEY: &str = "spk_dev_7b0e66c61ba21b0161d234fe962f8ea0c6e3163214585184";

pub(super) async fn require_admin(request: Request, next: Next) -> Response {
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .is_some_and(|(scheme, key)| {
            scheme.eq_ignore_ascii_case("Bearer")
                && bool::from(key.as_bytes().ct_eq(ADMIN_KEY.as_bytes()))
        });

    if !authorized {
        return (
            [(header::WWW_AUTHENTICATE, "Bearer")],
            ApiError::new(StatusCode::UNAUTHORIZED, "unauthorized"),
        )
            .into_response();
    }

    next.run(request).await
}

#[cfg(test)]
pub(super) fn test_authorization() -> String {
    format!("Bearer {ADMIN_KEY}")
}
