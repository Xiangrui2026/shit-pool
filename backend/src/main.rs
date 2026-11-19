mod config;
mod db;
mod http;
mod images;
mod questions;
mod render_jobs;
mod storage;
mod tex;

use std::{env, error::Error};

use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_env()?;
    let mut args = env::args().skip(1);
    let worker = match (args.next().as_deref(), args.next()) {
        (None, None) => false,
        (Some("render-worker"), None) => true,
        _ => return Err("Usage: shit-pool-api [render-worker]".into()),
    };
    let database = config
        .database_url
        .as_deref()
        .map(|url| db::connection_pool(url, config.database_max_connections))
        .transpose()
        .map_err(|_| "Invalid DATABASE_URL")?;
    let images = images::ImageStore::new(storage::LocalFileStore::new(config.image_directory));
    let render = config.render_enabled.then(|| {
        render_jobs::Artifacts::new(storage::LocalFileStore::new(config.render_directory))
    });
    if worker {
        return render_jobs::run(
            database.ok_or("DATABASE_URL is required for render-worker")?,
            images,
            render.ok_or("Set SHIT_POOL_RENDER_ENABLED=true for render-worker")?,
            config.render_image,
        )
        .await;
    }
    if let Some(pool) = &database {
        db::migrate(pool).await?;
    }

    let listener = tokio::net::TcpListener::bind(config.api_address).await?;
    println!(
        "SHIT-Pool API listening on http://{}",
        listener.local_addr()?
    );
    axum::serve(listener, http::router(database, images, render))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
