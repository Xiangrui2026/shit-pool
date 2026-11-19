use std::io;

use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};

use super::{
    AppState,
    error::ApiError,
    questions::{not_found, valid_id},
};
use crate::{
    images::{ImageStore, mime_type},
    questions::{self, QuestionDetail},
    tex,
};

const MAX_ARCHIVE_IMAGE_BYTES: usize = 32 * 1024 * 1024;
const MAX_ARCHIVE_IMAGES: usize = 32;

pub(super) async fn download(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Response, ApiError> {
    let id = valid_id(id)?;
    let question = questions::find(state.pool()?, id)
        .await?
        .ok_or_else(not_found)?;
    let data = tokio::task::spawn_blocking(move || archive(&question, &state.images))
        .await
        .map_err(|_| ApiError::image_storage())??;
    let filename = format!("attachment; filename=\"question-{id}.zip\"");
    Ok((
        [
            (header::CONTENT_TYPE, "application/zip"),
            (header::CONTENT_DISPOSITION, filename.as_str()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        data,
    )
        .into_response())
}

fn archive(detail: &QuestionDetail, images: &ImageStore) -> Result<Vec<u8>, ApiError> {
    let stem = &detail.question.stem;
    let solutions: Vec<_> = detail
        .solutions
        .0
        .iter()
        .map(|solution| tex::AnswerSection {
            title: &solution.title,
            body: &solution.body,
            is_default: solution.is_default,
        })
        .collect();
    let names = tex::image_references(stem, &solutions)
        .into_iter()
        .map(|path| {
            path.strip_prefix("images/")
                .filter(|name| mime_type(name).is_some())
                .ok_or_else(|| unprocessable("unsupported_image_reference"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if names.len() > MAX_ARCHIVE_IMAGES {
        return Err(unprocessable("archive_too_large"));
    }
    let mut remaining = MAX_ARCHIVE_IMAGE_BYTES;
    let mut output = tex::Archive::new(detail.question.id, stem, &solutions)
        .map_err(|_| ApiError::image_storage())?;
    for name in names {
        let data = images
            .read(name, remaining)
            .map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => unprocessable("missing_question_image"),
                io::ErrorKind::FileTooLarge => unprocessable("archive_too_large"),
                _ => ApiError::image_storage(),
            })?;
        remaining -= data.len();
        output
            .add_image(name, &data)
            .map_err(|_| ApiError::image_storage())?;
    }
    output.finish().map_err(|_| ApiError::image_storage())
}

fn unprocessable(code: &'static str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, code)
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use axum::body::to_bytes;

    use super::*;
    use crate::{
        images::test_png,
        questions::{Question, Solution},
        storage::LocalFileStore,
    };

    fn question(stem: String, answer: String) -> QuestionDetail {
        QuestionDetail {
            question: Question {
                id: 42,
                category: "数学".into(),
                source: "样题".into(),
                original_number: None,
                paper_source: "样题".into(),
                stem,
                answer: answer.clone(),
                notes: String::new(),
                tags: String::new(),
            },
            solutions: sqlx::types::Json(vec![Solution {
                id: 1,
                title: "默认解法".into(),
                body: answer,
                is_default: true,
            }]),
        }
    }

    #[test]
    fn passes_stem_and_solution_images_to_the_archive() {
        let directory = tempfile::tempdir().unwrap();
        let images = ImageStore::new(LocalFileStore::new(directory.path().into()));
        let png = test_png();
        let name = images.save(&png).unwrap();
        let source = question(
            format!(r"题干 \includegraphics{{images/{name}}}"),
            format!(r"解答 \includegraphics{{images/{name}}}"),
        );
        let bytes = archive(&source, &images).unwrap();
        let mut zip = zip::ZipArchive::new(io::Cursor::new(bytes)).unwrap();
        assert_eq!(zip.len(), 2);
        let mut data = Vec::new();
        zip.by_name(&format!("images/{name}"))
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, png);
    }

    #[tokio::test]
    async fn refuses_unsupported_or_missing_images() {
        let directory = tempfile::tempdir().unwrap();
        let images = ImageStore::new(LocalFileStore::new(directory.path().into()));
        for (stem, code) in [
            (
                r"\includegraphics{../local.png}".to_owned(),
                "unsupported_image_reference",
            ),
            (
                format!(r"\includegraphics{{images/{}.png}}", "0".repeat(64)),
                "missing_question_image",
            ),
        ] {
            let source = question(stem, "$2$".into());
            let response = archive(&source, &images).unwrap_err().into_response();
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            let body = to_bytes(response.into_body(), 1024).await.unwrap();
            assert_eq!(body.as_ref(), format!(r#"{{"error":"{code}"}}"#).as_bytes());
        }
    }

    #[tokio::test]
    async fn rejects_too_many_images_before_reading_them() {
        let directory = tempfile::tempdir().unwrap();
        let images = ImageStore::new(LocalFileStore::new(directory.path().into()));
        let stem = (0..=MAX_ARCHIVE_IMAGES)
            .map(|index| format!(r"\includegraphics{{images/{index:064x}.png}}"))
            .collect::<Vec<_>>()
            .join("\n");
        let source = question(stem, "$2$".into());
        let response = archive(&source, &images).unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(body.as_ref(), br#"{"error":"archive_too_large"}"#);
    }
}
