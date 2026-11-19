use std::{env, error::Error, io, net::SocketAddr, num::NonZeroU32, path::PathBuf};

pub(crate) struct Config {
    pub api_address: SocketAddr,
    pub database_url: Option<String>,
    pub database_max_connections: u32,
    pub image_directory: PathBuf,
    pub render_enabled: bool,
    pub render_directory: PathBuf,
    pub render_image: String,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self, Box<dyn Error>> {
        match dotenvy::dotenv() {
            Ok(_) => (),
            Err(dotenvy::Error::Io(error)) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }

        let api_address = match optional_var("API_ADDR")? {
            Some(address) => address.parse()?,
            None => SocketAddr::from(([127, 0, 0, 1], 3000)),
        };

        Ok(Self {
            api_address,
            database_url: optional_var("DATABASE_URL")?,
            image_directory: optional_var("SHIT_POOL_IMAGE_DIR")?
                .map_or_else(|| PathBuf::from("data/images"), PathBuf::from),
            render_enabled: match optional_var("SHIT_POOL_RENDER_ENABLED")?.as_deref() {
                None | Some("false") => false,
                Some("true") => true,
                _ => return Err("SHIT_POOL_RENDER_ENABLED must be true or false".into()),
            },
            render_directory: optional_var("SHIT_POOL_RENDER_DIR")?
                .map_or_else(|| PathBuf::from("data/render"), PathBuf::from),
            render_image: optional_var("SHIT_POOL_RENDER_IMAGE")?
                .unwrap_or_else(|| "shit-pool-ccho:1".into()),
            database_max_connections: match optional_var("DATABASE_MAX_CONNECTIONS")? {
                Some(value) => value
                    .parse::<NonZeroU32>()
                    .map_err(|_| "DATABASE_MAX_CONNECTIONS must be a positive integer")?
                    .get(),
                None => 5,
            },
        })
    }
}

fn optional_var(key: &str) -> Result<Option<String>, env::VarError> {
    match env::var(key) {
        Ok(value) if value.trim().is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error),
    }
}
