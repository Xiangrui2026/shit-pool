use std::{
    error::Error,
    io::{self, Cursor, Write},
    process::Stdio,
    time::Duration,
};

use sqlx::PgPool;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    time::{sleep, timeout},
};
use zip::{ZipWriter, write::SimpleFileOptions};

use super::{Artifact, Artifacts, MAX_PDF_BYTES, Snapshot, repository, template};
use crate::images::ImageStore;

const MAX_IMAGE_BYTES: usize = 24 * 1024 * 1024;
const COMPILE_TIMEOUT: Duration = Duration::from_secs(90);

pub(crate) async fn run(
    pool: PgPool,
    images: ImageStore,
    artifacts: Artifacts,
    image: String,
) -> Result<(), Box<dyn Error>> {
    let engine = ContainerEngine::new(image)?;
    engine.check().await?;
    println!("CChO render worker ready");
    loop {
        for id in repository::expire(&pool).await? {
            let artifacts = artifacts.clone();
            let key = id.clone();
            match tokio::task::spawn_blocking(move || artifacts.remove(&key)).await? {
                Ok(()) => repository::forget(&pool, &id).await?,
                Err(error) => eprintln!("Could not remove expired render artifacts: {error}"),
            }
        }
        if let Some(job) = repository::claim(&pool).await? {
            let result = render(&job.id, &job.snapshot.0, &images, &artifacts, &engine).await;
            if let Err(code) = result {
                eprintln!("Render job failed: {code}");
                let artifacts = artifacts.clone();
                let id = job.id.clone();
                let _ = tokio::task::spawn_blocking(move || artifacts.remove(&id)).await;
            }
            repository::finish(&pool, &job.id, result.err()).await?;
        } else {
            sleep(Duration::from_secs(2)).await;
        }
    }
}

async fn render(
    id: &str,
    snapshot: &Snapshot,
    images: &ImageStore,
    artifacts: &Artifacts,
    engine: &ContainerEngine,
) -> Result<(), &'static str> {
    for artifact in [Artifact::Questions, Artifact::Answers] {
        if matches!(artifact, Artifact::Answers) && !snapshot.has_answers() {
            continue;
        }
        let paper = template::prepare(snapshot, artifact)?;
        let images = images.clone();
        let input = tokio::task::spawn_blocking(move || bundle(paper, &images))
            .await
            .map_err(|_| "render_storage_error")??;
        let pdf = engine.compile(id, artifact, input).await?;
        let artifacts = artifacts.clone();
        let key = id.to_owned();
        tokio::task::spawn_blocking(move || artifacts.save(&key, artifact, &pdf))
            .await
            .map_err(|_| "render_storage_error")?
            .map_err(|_| "render_storage_error")?;
    }
    Ok(())
}

fn bundle(paper: template::PaperInput, images: &ImageStore) -> Result<Vec<u8>, &'static str> {
    let mut bundle = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in paper.sources {
        add_file(&mut bundle, &name, &data).map_err(|_| "render_storage_error")?;
    }
    let mut remaining = MAX_IMAGE_BYTES;
    for name in paper.images {
        let data = images
            .read(&name, remaining)
            .map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => "missing_question_image",
                io::ErrorKind::FileTooLarge => "render_images_too_large",
                _ => "render_storage_error",
            })?;
        remaining -= data.len();
        add_file(&mut bundle, &format!("images/{name}"), &data)
            .map_err(|_| "render_storage_error")?;
    }
    Ok(bundle
        .finish()
        .map_err(|_| "render_storage_error")?
        .into_inner())
}

fn add_file(bundle: &mut ZipWriter<Cursor<Vec<u8>>>, name: &str, data: &[u8]) -> io::Result<()> {
    bundle.start_file(name, SimpleFileOptions::default())?;
    bundle.write_all(data)
}

struct ContainerEngine(String);

impl ContainerEngine {
    fn new(image: String) -> Result<Self, Box<dyn Error>> {
        if image.starts_with('-')
            || !image.bytes().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':' | b'@')
            })
        {
            return Err("Invalid SHIT_POOL_RENDER_IMAGE".into());
        }
        Ok(Self(image))
    }

    async fn check(&self) -> Result<(), Box<dyn Error>> {
        let mut command = Command::new("docker");
        command
            .args(["image", "inspect", &self.0])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if !matches!(timeout(Duration::from_secs(10), command.status()).await, Ok(Ok(status)) if status.success())
        {
            return Err(
                "CChO worker needs a running Docker engine and a locally built renderer image"
                    .into(),
            );
        }
        Ok(())
    }

    async fn compile(
        &self,
        id: &str,
        artifact: Artifact,
        input: Vec<u8>,
    ) -> Result<Vec<u8>, &'static str> {
        let name = format!("shit-pool-render-{id}-{}", artifact.name());
        let ci_debug = std::env::var_os("SHIT_POOL_CI_TEX_DEBUG").is_some_and(|value| value == "1");
        let mut child = Command::new("docker")
            .args([
                "run",
                "--rm",
                "--pull=never",
                "--interactive",
                "--name",
                &name,
                "--network=none",
                "--read-only",
                "--cap-drop=ALL",
                "--security-opt=no-new-privileges",
                "--user=65534:65534",
                "--pids-limit=64",
                "--cpus=0.75",
                "--memory=512m",
                "--memory-swap=512m",
                "--log-driver=none",
                "--env=SHIT_POOL_CI_TEX_DEBUG",
                "--tmpfs=/work:rw,nosuid,nodev,noexec,size=80m,mode=1777",
                "--tmpfs=/tmp:rw,nosuid,nodev,noexec,size=32m,mode=1777",
                &self.0,
                artifact.name(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(if ci_debug {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| "render_engine_unavailable")?;
        let mut stdin = child.stdin.take().ok_or("render_engine_unavailable")?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or("render_engine_unavailable")?
            .take(MAX_PDF_BYTES as u64 + 1);
        let result = timeout(COMPILE_TIMEOUT, async {
            let send = async move {
                stdin.write_all(&input).await?;
                stdin.shutdown().await
            };
            let receive = async move {
                let mut pdf = Vec::new();
                stdout.read_to_end(&mut pdf).await?;
                if pdf.len() > MAX_PDF_BYTES {
                    return Err(io::ErrorKind::FileTooLarge.into());
                }
                Ok(pdf)
            };
            let (_, pdf) = tokio::try_join!(send, receive)?;
            if !child.wait().await?.success() || !pdf.starts_with(b"%PDF-") {
                return Err(io::ErrorKind::InvalidData.into());
            }
            Ok::<Vec<u8>, io::Error>(pdf)
        })
        .await;
        match result {
            Ok(Ok(pdf)) => Ok(pdf),
            error => {
                let _ = child.start_kill();
                let _ = timeout(
                    Duration::from_secs(5),
                    Command::new("docker")
                        .args(["rm", "-f", &name])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .kill_on_drop(true)
                        .status(),
                )
                .await;
                match error {
                    Err(_) => Err("render_timeout"),
                    Ok(Err(error)) if error.kind() == io::ErrorKind::FileTooLarge => {
                        Err("render_pdf_too_large")
                    }
                    _ => Err("render_compile_failed"),
                }
            }
        }
    }
}
