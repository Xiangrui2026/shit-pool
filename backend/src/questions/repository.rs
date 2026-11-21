use sqlx::{PgPool, types::Json};

use super::{
    Category, NewQuestion, Paper, Question, QuestionDetail, QuestionPatch, SelectedQuestion, Tag,
    solutions,
};

pub(crate) async fn categories(pool: &PgPool) -> Result<Vec<Category>, sqlx::Error> {
    sqlx::query_as(
        "SELECT c.id, c.name, count(q.id) AS question_count
         FROM categories AS c LEFT JOIN questions AS q ON q.category_id = c.id AND q.deleted_at IS NULL
         GROUP BY c.id, c.name ORDER BY c.name, c.id",
    )
    .fetch_all(pool)
    .await
}

pub(crate) async fn tags(pool: &PgPool) -> Result<Vec<Tag>, sqlx::Error> {
    sqlx::query_as(
        "SELECT split.name, count(*) AS question_count
         FROM questions AS q
         CROSS JOIN LATERAL unnest(string_to_array(q.tags, ',')) AS split(name)
         WHERE q.deleted_at IS NULL AND split.name <> ''
         GROUP BY split.name
         ORDER BY question_count DESC, split.name",
    )
    .fetch_all(pool)
    .await
}

pub(crate) async fn search(
    pool: &PgPool,
    keyword: &str,
    category_id: Option<i32>,
    offset: i64,
    limit: i64,
    tags: &[&str],
    match_all_tags: bool,
) -> Result<(i64, Vec<Question>), sqlx::Error> {
    let (total, Json(items)): (i64, Json<Vec<Question>>) = sqlx::query_as(
        "WITH matched AS (
             SELECT q.id FROM questions AS q
             WHERE q.deleted_at IS NULL
               AND ($2::integer IS NULL OR q.category_id = $2)
               AND ($1::text = '' OR strpos(lower(q.stem), lower($1)) > 0
                                OR strpos(lower(q.source), lower($1)) > 0)
               AND (cardinality($5::text[]) = 0 OR
                    CASE WHEN $6::boolean THEN string_to_array(q.tags, ',') @> $5
                         ELSE string_to_array(q.tags, ',') && $5 END)
         ), visible AS (
             SELECT q.id, c.name AS category, q.source, q.original_number, q.paper_source,
                    q.stem, q.answer, q.notes, q.tags, q.score
             FROM (SELECT id FROM matched ORDER BY id DESC LIMIT $4 OFFSET $3) AS page
             JOIN question_papers AS q ON q.id = page.id
             JOIN categories AS c ON c.id = q.category_id
         )
         SELECT (SELECT count(*) FROM matched),
                COALESCE((SELECT json_agg(visible ORDER BY id DESC) FROM visible), '[]'::json)",
    )
    .bind(keyword)
    .bind(category_id)
    .bind(offset)
    .bind(limit)
    .bind(tags)
    .bind(match_all_tags)
    .fetch_one(pool)
    .await?;

    Ok((total, items))
}

pub(crate) async fn find(pool: &PgPool, id: i32) -> Result<Option<QuestionDetail>, sqlx::Error> {
    sqlx::query_as(
        "SELECT q.id, c.name AS category, q.source, q.original_number, q.paper_source,
                q.stem, q.answer, q.notes, q.tags, q.score,
                COALESCE((
                    SELECT json_agg(solution ORDER BY is_default DESC, id) FROM (
                        SELECT id, title, body,
                               row_number() OVER (ORDER BY is_preferred DESC, id) = 1 AS is_default
                        FROM question_solutions
                        WHERE question_id = q.id AND deleted_at IS NULL
                    ) AS solution
                ), '[]'::json) AS solutions
         FROM question_papers AS q JOIN categories AS c ON c.id = q.category_id
         WHERE q.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn papers(pool: &PgPool) -> Result<Vec<Paper>, sqlx::Error> {
    sqlx::query_as(
        "SELECT paper_source AS source, count(*) AS question_count
         FROM question_papers GROUP BY paper_source ORDER BY paper_source",
    )
    .fetch_all(pool)
    .await
}

pub(crate) async fn render_selection(
    pool: &PgPool,
    ids: &[i32],
) -> Result<Vec<SelectedQuestion>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, source, original_number, stem, answer
         FROM question_papers WHERE id = ANY($1)
         ORDER BY array_position($1, id)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await
}

pub(crate) async fn paper_questions(
    pool: &PgPool,
    source: &str,
) -> Result<Vec<Question>, sqlx::Error> {
    sqlx::query_as(
        "SELECT q.id, c.name AS category, q.source, q.original_number, q.paper_source,
                q.stem, q.answer, q.notes, q.tags, q.score
         FROM question_papers AS q JOIN categories AS c ON c.id = q.category_id
         WHERE q.paper_source = $1
         ORDER BY q.original_number ASC NULLS LAST, q.id ASC",
    )
    .bind(source)
    .fetch_all(pool)
    .await
}

pub(crate) async fn update(
    pool: &PgPool,
    id: i32,
    patch: QuestionPatch,
) -> Result<Option<i32>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let updated = sqlx::query_scalar(
        "WITH target AS (
             SELECT id FROM questions WHERE id = $1 AND deleted_at IS NULL FOR UPDATE
         ), category AS (
             INSERT INTO categories (name) SELECT $2::text FROM target WHERE $2 IS NOT NULL
             ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
             RETURNING id
         )
         UPDATE questions AS q
         SET category_id = COALESCE((SELECT id FROM category), q.category_id),
             source = COALESCE($3, q.source), stem = COALESCE($4, q.stem),
             notes = COALESCE($5, q.notes), tags = COALESCE($6, q.tags),
             original_number = CASE WHEN $7 THEN $8 ELSE q.original_number END
         FROM target WHERE q.id = target.id
         RETURNING q.id",
    )
    .bind(id)
    .bind(patch.category)
    .bind(patch.source)
    .bind(patch.stem)
    .bind(patch.notes)
    .bind(patch.tags)
    .bind(patch.original_number.is_some())
    .bind(patch.original_number.flatten())
    .fetch_optional(&mut *transaction)
    .await?;
    if let (Some(id), Some(answer)) = (updated, patch.answer) {
        solutions::write_legacy_answer(&mut transaction, id, answer).await?;
    }
    transaction.commit().await?;
    Ok(updated)
}

pub(crate) async fn soft_delete(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE questions SET deleted_at = CURRENT_TIMESTAMP
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn insert(pool: &PgPool, question: NewQuestion) -> Result<i32, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let id = sqlx::query_scalar(
        "WITH category AS (
             INSERT INTO categories (name) VALUES ($1)
             ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
             RETURNING id
         )
         INSERT INTO questions (category_id, source, stem, notes, tags, original_number)
         SELECT id, $2, $3, $4, $5, $6 FROM category
         RETURNING id",
    )
    .bind(question.category)
    .bind(question.source)
    .bind(question.stem)
    .bind(question.notes)
    .bind(question.tags)
    .bind(question.original_number)
    .fetch_one(&mut *transaction)
    .await?;
    if !question.answer.is_empty() {
        solutions::write_legacy_answer(&mut transaction, id, question.answer).await?;
    }
    transaction.commit().await?;
    Ok(id)
}
