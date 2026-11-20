use std::{env, error::Error, io, net::SocketAddr, num::NonZeroU32, path::PathBuf};

pub(crate) struct Config {
    pub api_address: SocketAddr,
    pub database_url: Option<String>,
    pub database_max_connections: u32,
    pub image_directory: PathBuf,
    pub admin_key: Option<String>,
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
            admin_key: validate_admin_key(optional_admin_key()?)?,
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

fn validate_admin_key(key: Option<String>) -> Result<Option<String>, &'static str> {
    if key
        .as_ref()
        .is_some_and(|key| key.len() < 32 || !key.bytes().all(|byte| byte.is_ascii_graphic()))
    {
        return Err(
            "SHIT_POOL_ADMIN_KEY must have at least 32 ASCII characters without whitespace",
        );
    }
    Ok(key)
}

fn optional_admin_key() -> Result<Option<String>, env::VarError> {
    match env::var("SHIT_POOL_ADMIN_KEY") {
        Ok(key) if key.is_empty() => Ok(None),
        Ok(key) => Ok(Some(key)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn optional_var(key: &str) -> Result<Option<String>, env::VarError> {
    match env::var(key) {
        Ok(value) if value.trim().is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_admin_key;

    #[test]
    fn permits_read_only_mode_and_rejects_weak_or_malformed_keys() {
        assert!(validate_admin_key(None).unwrap().is_none());
        assert!(validate_admin_key(Some("short".into())).is_err());
        assert!(validate_admin_key(Some("x".repeat(32) + " ")).is_err());
        assert!(validate_admin_key(Some("x".repeat(32))).is_ok());
    }
}
