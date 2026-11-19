mod papers;
mod solutions;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    routing::{get as route_get, patch as route_patch, post as route_post},
};
use serde::{Deserialize, Serialize};

use super::{AdminRoute, AppState, error::ApiError};
use crate::questions::{self, Category, NewQuestion, Question, QuestionDetail, QuestionPatch, Tag};

const PAGE_SIZE: i64 = 10;
const MAX_TAG_FILTER_LENGTH: usize = 4_000;
const MAX_JSON_BODY_BYTES: usize = 256 * 1024;

pub(super) fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/categories", route_get(categories))
        .route("/api/tags", route_get(tags))
        .route("/api/papers", route_get(papers::list))
        .route("/api/papers/questions", route_get(papers::detail))
        .route("/api/questions", route_get(search))
        .route("/api/questions/{id}", route_get(get))
}

pub(super) fn admin_routes() -> [AdminRoute; 4] {
    [
        ("/api/questions", route_post(create)),
        ("/api/questions/{id}", route_patch(update).delete(delete)),
        (
            "/api/questions/{question_id}/solutions",
            route_post(solutions::create),
        ),
        (
            "/api/questions/{question_id}/solutions/{id}",
            route_patch(solutions::update).delete(solutions::delete),
        ),
    ]
    .map(|(path, methods)| {
        (
            path,
            methods.layer(DefaultBodyLimit::max(MAX_JSON_BODY_BYTES)),
        )
    })
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TagMatch {
    #[default]
    Any,
    All,
}

#[derive(Deserialize)]
pub(super) struct SearchQuery {
    #[serde(default)]
    q: String,
    category_id: Option<i32>,
    page: Option<u32>,
    #[serde(default)]
    tags: String,
    #[serde(default)]
    tag_match: TagMatch,
}

#[derive(Serialize)]
pub(super) struct SearchPage {
    items: Vec<Question>,
    page: u32,
    page_size: i64,
    total: i64,
}

#[derive(Serialize)]
pub(super) struct QuestionId {
    id: i32,
}

pub(super) async fn categories(
    State(state): State<AppState>,
) -> Result<Json<Vec<Category>>, ApiError> {
    Ok(Json(questions::categories(state.pool()?).await?))
}

pub(super) async fn tags(State(state): State<AppState>) -> Result<Json<Vec<Tag>>, ApiError> {
    Ok(Json(questions::tags(state.pool()?).await?))
}

pub(super) async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchPage>, ApiError> {
    let keyword = query.q.trim();
    if keyword.chars().count() > 120 || keyword.contains('\0') {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "invalid_query"));
    }
    let page = query.page.unwrap_or(1);
    if page == 0 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "invalid_page"));
    }
    if query.category_id.is_some_and(|id| id < 1) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "invalid_category"));
    }
    if query.tags.len() > MAX_TAG_FILTER_LENGTH || query.tags.contains('\0') {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "invalid_tags"));
    }
    let tags = query
        .tags
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();

    let pool = state.pool()?;
    let (total, items) = questions::search(
        pool,
        keyword,
        query.category_id,
        i64::from(page - 1) * PAGE_SIZE,
        PAGE_SIZE,
        &tags,
        matches!(query.tag_match, TagMatch::All),
    )
    .await?;
    Ok(Json(SearchPage {
        items,
        page,
        page_size: PAGE_SIZE,
        total,
    }))
}

pub(super) async fn create(
    State(state): State<AppState>,
    Json(question): Json<NewQuestion>,
) -> Result<(StatusCode, Json<QuestionId>), ApiError> {
    let question = question
        .validated()
        .map_err(|code| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, code))?;
    let id = questions::insert(state.pool()?, question).await?;
    Ok((StatusCode::CREATED, Json(QuestionId { id })))
}

pub(super) async fn get(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<QuestionDetail>, ApiError> {
    let id = valid_id(id)?;
    questions::find(state.pool()?, id)
        .await?
        .map(Json)
        .ok_or_else(not_found)
}

pub(super) async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(patch): Json<QuestionPatch>,
) -> Result<Json<QuestionId>, ApiError> {
    let id = valid_id(id)?;
    let patch = patch
        .validated()
        .map_err(|code| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, code))?;
    let id = questions::update(state.pool()?, id, patch)
        .await?
        .ok_or_else(not_found)?;
    Ok(Json(QuestionId { id }))
}

pub(super) async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<QuestionId>, ApiError> {
    let id = valid_id(id)?;
    if !questions::soft_delete(state.pool()?, id).await? {
        return Err(not_found());
    }
    Ok(Json(QuestionId { id }))
}

pub(super) fn valid_id(id: i32) -> Result<i32, ApiError> {
    if id < 1 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "invalid_id"));
    }
    Ok(id)
}

pub(super) fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "question_not_found")
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
        response::Response,
    };
    use tower::ServiceExt;

    use crate::{
        http::{auth, router},
        images::ImageStore,
        storage::LocalFileStore,
    };

    const NEW_QUESTION: &str = r#"{"category":"数学","source":"测试卷","stem":"$x+1$","answer":"$2$","tags":"代数，计算","notes":"备注"}"#;

    #[tokio::test]
    async fn only_mutations_require_a_valid_bearer_key() {
        let directory = tempfile::tempdir().unwrap();
        let app = router(
            None,
            ImageStore::new(LocalFileStore::new(directory.path().into())),
            None,
        );
        for path in [
            "/api/questions?q=test",
            "/api/questions/1",
            "/api/questions/1/export",
            "/api/categories",
            "/api/tags",
            "/api/papers",
            "/api/papers/questions?source=test",
        ] {
            let response = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        }

        for authorization in [None, Some("Bearer wrong-key"), Some("Basic wrong-key")] {
            for (method, path) in [
                ("POST", "/api/questions"),
                ("PATCH", "/api/questions/1"),
                ("DELETE", "/api/questions/1"),
                ("POST", "/api/questions/1/solutions"),
                ("PATCH", "/api/questions/1/solutions/2"),
                ("DELETE", "/api/questions/1/solutions/2"),
            ] {
                let response = write_question(app.clone(), method, path, authorization, "{").await;
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
                assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
            }
        }

        let authorization = auth::test_authorization();
        let response = create_question(app, Some(&authorization), NEW_QUESTION).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_error(response, "database_not_configured").await;
    }

    #[tokio::test]
    async fn unsupported_requests_keep_the_routing_status_without_a_key() {
        let directory = tempfile::tempdir().unwrap();
        let app = router(
            None,
            ImageStore::new(LocalFileStore::new(directory.path().into())),
            None,
        );
        let response = write_question(app.clone(), "PUT", "/api/questions/1", None, "").await;
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        let response = write_question(app, "POST", "/api/unknown", None, "").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn question_writes_still_enforce_the_json_body_limit() {
        let directory = tempfile::tempdir().unwrap();
        let app = router(
            None,
            ImageStore::new(LocalFileStore::new(directory.path().into())),
            None,
        );
        let authorization = auth::test_authorization();
        let large_json = " ".repeat(super::MAX_JSON_BODY_BYTES + 1);
        for (method, path) in [
            ("POST", "/api/questions"),
            ("PATCH", "/api/questions/1"),
            ("POST", "/api/questions/1/solutions"),
            ("PATCH", "/api/questions/1/solutions/2"),
        ] {
            let response =
                write_question(app.clone(), method, path, Some(&authorization), &large_json).await;
            assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        }
    }

    #[tokio::test]
    async fn validates_solutions_and_their_parent_ids() {
        let directory = tempfile::tempdir().unwrap();
        let app = router(
            None,
            ImageStore::new(LocalFileStore::new(directory.path().into())),
            None,
        );
        let authorization = auth::test_authorization();
        for (method, path, body, expected, code) in [
            (
                "POST",
                "/api/questions/1/solutions",
                r#"{"title":" ","body":"$2$"}"#,
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_solution_title",
            ),
            (
                "POST",
                "/api/questions/1/solutions",
                r#"{"title":"解法","body":" \n "}"#,
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_solution_body",
            ),
            (
                "PATCH",
                "/api/questions/1/solutions/2",
                "{}",
                StatusCode::UNPROCESSABLE_ENTITY,
                "empty_patch",
            ),
            (
                "PATCH",
                "/api/questions/1/solutions/2",
                r#"{"make_default":true}"#,
                StatusCode::SERVICE_UNAVAILABLE,
                "database_not_configured",
            ),
            (
                "DELETE",
                "/api/questions/1/solutions/0",
                "",
                StatusCode::BAD_REQUEST,
                "invalid_id",
            ),
        ] {
            let response =
                write_question(app.clone(), method, path, Some(&authorization), body).await;
            assert_eq!(response.status(), expected);
            assert_error(response, code).await;
        }
    }

    #[tokio::test]
    async fn patch_rejects_null_or_empty_objects_and_accepts_clearing_optional_fields() {
        let directory = tempfile::tempdir().unwrap();
        let app = router(
            None,
            ImageStore::new(LocalFileStore::new(directory.path().into())),
            None,
        );
        let authorization = auth::test_authorization();
        for (body, expected) in [
            ("{}", StatusCode::UNPROCESSABLE_ENTITY),
            (r#"{"notes":null}"#, StatusCode::UNPROCESSABLE_ENTITY),
            (r#"{"notes":"","tags":""}"#, StatusCode::SERVICE_UNAVAILABLE),
        ] {
            let response = write_question(
                app.clone(),
                "PATCH",
                "/api/questions/1",
                Some(&authorization),
                body,
            )
            .await;
            assert_eq!(response.status(), expected);
        }
    }

    #[tokio::test]
    async fn validates_create_and_search_input_before_touching_the_database() {
        let directory = tempfile::tempdir().unwrap();
        let app = router(
            None,
            ImageStore::new(LocalFileStore::new(directory.path().into())),
            None,
        );
        let authorization = auth::test_authorization();
        let response = create_question(
            app.clone(),
            Some(&authorization),
            &NEW_QUESTION.replace("测试卷", " "),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_error(response, "invalid_source").await;

        for (path, code) in [
            ("/api/questions?category_id=0", "invalid_category"),
            ("/api/questions?tags=bad%00tag", "invalid_tags"),
            ("/api/questions?page=0", "invalid_page"),
            ("/api/papers/questions?source=%20", "invalid_source"),
        ] {
            let response = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert_error(response, code).await;
        }
    }

    async fn create_question(app: Router, authorization: Option<&str>, body: &str) -> Response {
        write_question(app, "POST", "/api/questions", authorization, body).await
    }

    async fn write_question(
        app: Router,
        method: &str,
        path: &str,
        authorization: Option<&str>,
        body: &str,
    ) -> Response {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(value) = authorization {
            request = request.header(header::AUTHORIZATION, value);
        }
        app.oneshot(request.body(Body::from(body.to_owned())).unwrap())
            .await
            .unwrap()
    }

    async fn assert_error(response: Response, code: &str) {
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(body.as_ref(), format!(r#"{{"error":"{code}"}}"#).as_bytes());
    }
}
