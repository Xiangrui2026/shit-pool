use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(2);
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

pub(crate) fn connection_pool(url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(CONNECTION_TIMEOUT)
        .connect_lazy(url)
}

pub(crate) async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

pub(crate) async fn ping(pool: &PgPool) -> bool {
    matches!(
        tokio::time::timeout(CONNECTION_TIMEOUT, sqlx::query("SELECT 1").execute(pool)).await,
        Ok(Ok(_))
    )
}
