use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};

use super::{checked, latex_source, present_string, validate};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewSolution {
    title: String,
    body: String,
    #[serde(default)]
    make_default: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct SolutionPatch {
    #[serde(deserialize_with = "present_string")]
    title: Option<String>,
    #[serde(deserialize_with = "present_string")]
    body: Option<String>,
    make_default: bool,
}

impl NewSolution {
    pub(crate) fn validated(self) -> Result<Self, &'static str> {
        Ok(Self {
            title: validate(self.title, 120, false).ok_or("invalid_solution_title")?,
            body: latex_source(self.body, 8000).ok_or("invalid_solution_body")?,
            make_default: self.make_default,
        })
    }
}

impl SolutionPatch {
    pub(crate) fn validated(self) -> Result<Self, &'static str> {
        if self.title.is_none() && self.body.is_none() && !self.make_default {
            return Err("empty_patch");
        }
        Ok(Self {
            title: checked(
                self.title,
                |value| validate(value, 120, false),
                "invalid_solution_title",
            )?,
            body: checked(
                self.body,
                |value| latex_source(value, 8000),
                "invalid_solution_body",
            )?,
            make_default: self.make_default,
        })
    }
}

pub(crate) async fn add_solution(
    pool: &PgPool,
    question_id: i32,
    solution: NewSolution,
) -> Result<Option<i32>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    if !lock_question(&mut transaction, question_id).await? {
        return Ok(None);
    }
    if solution.make_default {
        clear_preferred(&mut transaction, question_id).await?;
    }
    let id = sqlx::query_scalar(
        "INSERT INTO question_solutions (question_id, title, body, is_preferred)
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(question_id)
    .bind(solution.title)
    .bind(solution.body)
    .bind(solution.make_default)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(Some(id))
}

pub(crate) async fn update_solution(
    pool: &PgPool,
    question_id: i32,
    id: i32,
    patch: SolutionPatch,
) -> Result<bool, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    if !lock_question(&mut transaction, question_id).await? {
        return Ok(false);
    }
    if patch.make_default {
        clear_preferred(&mut transaction, question_id).await?;
    }
    let result = sqlx::query(
        "UPDATE question_solutions
         SET title = COALESCE($3, title), body = COALESCE($4, body),
             is_preferred = is_preferred OR $5
         WHERE question_id = $1 AND id = $2 AND deleted_at IS NULL",
    )
    .bind(question_id)
    .bind(id)
    .bind(patch.title)
    .bind(patch.body)
    .bind(patch.make_default)
    .execute(&mut *transaction)
    .await?;
    if result.rows_affected() == 0 {
        transaction.rollback().await?;
        return Ok(false);
    }
    transaction.commit().await?;
    Ok(true)
}

pub(crate) async fn delete_solution(
    pool: &PgPool,
    question_id: i32,
    id: i32,
) -> Result<bool, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    if !lock_question(&mut transaction, question_id).await? {
        return Ok(false);
    }
    let result = sqlx::query(
        "UPDATE question_solutions SET deleted_at = CURRENT_TIMESTAMP
         WHERE question_id = $1 AND id = $2 AND deleted_at IS NULL",
    )
    .bind(question_id)
    .bind(id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(result.rows_affected() == 1)
}

pub(super) async fn write_legacy_answer(
    transaction: &mut Transaction<'_, Postgres>,
    question_id: i32,
    answer: String,
) -> Result<(), sqlx::Error> {
    let id: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM question_solutions WHERE question_id = $1 AND deleted_at IS NULL
         ORDER BY is_preferred DESC, id LIMIT 1",
    )
    .bind(question_id)
    .fetch_optional(&mut **transaction)
    .await?;
    match (id, answer.is_empty()) {
        (Some(id), true) => {
            sqlx::query(
                "UPDATE question_solutions SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1",
            )
            .bind(id)
            .execute(&mut **transaction)
            .await?;
        }
        (Some(id), false) => {
            sqlx::query("UPDATE question_solutions SET body = $2 WHERE id = $1")
                .bind(id)
                .bind(answer)
                .execute(&mut **transaction)
                .await?;
        }
        (None, false) => {
            sqlx::query(
                "INSERT INTO question_solutions (question_id, title, body)
                 VALUES ($1, '默认解答', $2)",
            )
            .bind(question_id)
            .bind(answer)
            .execute(&mut **transaction)
            .await?;
        }
        (None, true) => {}
    }
    Ok(())
}

async fn lock_question(
    transaction: &mut Transaction<'_, Postgres>,
    id: i32,
) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query_scalar::<_, i32>(
        "SELECT id FROM questions WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut **transaction)
    .await?
    .is_some())
}

async fn clear_preferred(
    transaction: &mut Transaction<'_, Postgres>,
    question_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE question_solutions SET is_preferred = FALSE
         WHERE question_id = $1 AND is_preferred AND deleted_at IS NULL",
    )
    .bind(question_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
