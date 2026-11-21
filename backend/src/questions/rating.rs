use serde::{Deserialize, Serialize};
use sqlx::{PgPool, QueryBuilder};

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Vote {
    Star,
    Dislike,
}

#[derive(Serialize, sqlx::FromRow)]
pub(crate) struct QuestionScore {
    pub id: i32,
    pub score: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RankOrder {
    Good,
    Bad,
}

#[derive(Serialize, sqlx::FromRow)]
pub(crate) struct RankedQuestion {
    pub id: i32,
    pub category: String,
    pub source: String,
    pub stem: String,
    pub score: i64,
}

pub(crate) async fn rate(
    pool: &PgPool,
    id: i32,
    vote: Vote,
) -> Result<Option<QuestionScore>, sqlx::Error> {
    let delta = match vote {
        Vote::Star => 1_i64,
        Vote::Dislike => -1,
    };
    sqlx::query_as(
        "UPDATE questions SET score = score + $2
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING id, score",
    )
    .bind(id)
    .bind(delta)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn ranking(
    pool: &PgPool,
    order: RankOrder,
    tags: &[&str],
    match_all_tags: bool,
) -> Result<Vec<RankedQuestion>, sqlx::Error> {
    // Both directions use the same (score, id) index.
    let direction = match order {
        RankOrder::Good => " DESC",
        RankOrder::Bad => " ASC",
    };
    let mut query = QueryBuilder::new(
        "WITH ranked AS MATERIALIZED (
            SELECT id, score FROM questions WHERE deleted_at IS NULL",
    );
    if !tags.is_empty() {
        query
            .push(if match_all_tags {
                " AND string_to_array(tags, ',') @> "
            } else {
                " AND string_to_array(tags, ',') && "
            })
            .push_bind(tags)
            .push("::text[]");
    }
    query
        .push(" ORDER BY score")
        .push(direction)
        .push(", id")
        .push(direction)
        .push(
            " LIMIT 25
         )
         SELECT q.id, c.name AS category, q.source, q.stem, ranked.score
         FROM ranked
         JOIN questions AS q ON q.id = ranked.id
         JOIN categories AS c ON c.id = q.category_id
         ORDER BY ranked.score",
        )
        .push(direction)
        .push(", ranked.id")
        .push(direction);

    query.build_query_as().fetch_all(pool).await
}
