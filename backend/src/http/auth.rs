use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;

use super::error::ApiError;

pub(super) async fn require_admin(
    State(admin_key): State<Option<Arc<str>>>,
    request: Request,
    next: Next,
) -> Response {
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .is_some_and(|(scheme, key)| {
            scheme.eq_ignore_ascii_case("Bearer")
                && admin_key
                    .as_ref()
                    .is_some_and(|expected| bool::from(key.as_bytes().ct_eq(expected.as_bytes())))
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
pub(super) const TEST_KEY: &str = "test-only-key-for-admin-route-tests";

#[cfg(test)]
pub(super) fn test_authorization() -> String {
    format!("Bearer {TEST_KEY}")
}
