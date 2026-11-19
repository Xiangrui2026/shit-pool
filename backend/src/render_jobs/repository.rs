use serde::Serialize;
use sqlx::{PgPool, types::Json};

use super::Snapshot;

#[derive(Serialize, sqlx::FromRow)]
pub(crate) struct JobStatus {
    pub id: String,
    pub status: String,
    pub has_answers: bool,
    pub error: Option<String>,
}

#[derive(sqlx::FromRow)]
pub(super) struct ClaimedJob {
    pub id: String,
    pub snapshot: Json<Snapshot>,
}

pub(crate) async fn enqueue(
    pool: &PgPool,
    id: &str,
    snapshot: Snapshot,
) -> Result<Option<JobStatus>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(102938471, 1)")
        .execute(&mut *transaction)
        .await?;
    let has_answers = snapshot.has_answers();
    let job = sqlx::query_as(
        "INSERT INTO render_jobs (id, snapshot, has_answers)
         SELECT $1, $2, $3
         WHERE (SELECT count(*) FROM render_jobs WHERE status IN ('queued', 'running')) < 12
           AND (SELECT count(*) FROM render_jobs WHERE created_at > now() - interval '24 hours') < 60
         RETURNING id, status, has_answers, error",
    )
    .bind(id)
    .bind(Json(snapshot))
    .bind(has_answers)
    .fetch_optional(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(job)
}

pub(crate) async fn status(pool: &PgPool, id: &str) -> Result<Option<JobStatus>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, status, has_answers, error FROM render_jobs
         WHERE id = $1 AND (finished_at IS NULL OR finished_at > now() - interval '24 hours')",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub(super) async fn claim(pool: &PgPool) -> Result<Option<ClaimedJob>, sqlx::Error> {
    sqlx::query_as(
        "WITH next AS (
             SELECT id FROM render_jobs
             WHERE status = 'queued' AND created_at > now() - interval '30 minutes'
             ORDER BY created_at, id LIMIT 1 FOR UPDATE SKIP LOCKED
         )
         UPDATE render_jobs AS jobs SET status = 'running', started_at = now()
         FROM next WHERE jobs.id = next.id RETURNING jobs.id, jobs.snapshot",
    )
    .fetch_optional(pool)
    .await
}

pub(super) async fn finish(
    pool: &PgPool,
    id: &str,
    error: Option<&'static str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE render_jobs SET status = $2, error = $3, finished_at = now()
         WHERE id = $1 AND status = 'running'",
    )
    .bind(id)
    .bind(if error.is_some() {
        "failed"
    } else {
        "completed"
    })
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn expire(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query(
        "UPDATE render_jobs SET status = 'failed', finished_at = now(),
             error = CASE WHEN status = 'queued' THEN 'queue_timeout' ELSE 'render_interrupted' END
         WHERE (status = 'queued' AND created_at <= now() - interval '30 minutes')
            OR (status = 'running' AND started_at <= now() - interval '5 minutes')",
    )
    .execute(pool)
    .await?;
    sqlx::query_scalar(
        "SELECT id FROM render_jobs WHERE finished_at <= now() - interval '24 hours'
         ORDER BY finished_at LIMIT 32",
    )
    .fetch_all(pool)
    .await
}

pub(super) async fn forget(pool: &PgPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM render_jobs WHERE id = $1 AND finished_at <= now() - interval '24 hours'",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
