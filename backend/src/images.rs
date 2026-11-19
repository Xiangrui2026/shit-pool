use std::{
    io::{self, Cursor},
    sync::Arc,
};

use image::{ImageFormat, ImageReader, Limits};
use sha2::{Digest, Sha256};

use crate::storage::{FileKey, FileStore};

pub(crate) const MAX_IMAGE_SIZE: usize = 8 * 1024 * 1024;
const MAX_DIMENSION: u32 = 4096;

#[derive(Clone)]
pub(crate) struct ImageStore(Arc<dyn FileStore>);

#[derive(Debug)]
pub(crate) enum ImageError {
    Unsupported,
    Invalid,
    TooLarge,
    Io,
}

impl ImageStore {
    pub(crate) fn new(storage: impl FileStore + 'static) -> Self {
        Self(Arc::new(storage))
    }

    pub(crate) fn save(&self, data: &[u8]) -> Result<String, ImageError> {
        if data.len() > MAX_IMAGE_SIZE {
            return Err(ImageError::TooLarge);
        }
        let extension = validate_image(data)?;
        let name = format!("{:x}.{extension}", Sha256::digest(data));
        let key = FileKey::new(&name).ok_or(ImageError::Io)?;
        self.0
            .put_if_absent(key, data)
            .map_err(|_| ImageError::Io)?;
        Ok(name)
    }

    pub(crate) fn read(&self, name: &str, max_bytes: usize) -> io::Result<Vec<u8>> {
        if mime_type(name).is_none() {
            return Err(io::ErrorKind::NotFound.into());
        }
        let key = FileKey::new(name).ok_or(io::ErrorKind::NotFound)?;
        self.0.read(key, max_bytes.min(MAX_IMAGE_SIZE))
    }
}

pub(crate) fn mime_type(name: &str) -> Option<&'static str> {
    let (hash, extension) = name.split_once('.')?;
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return None;
    }
    match extension {
        "png" => Some("image/png"),
        "jpg" => Some("image/jpeg"),
        _ => None,
    }
}

fn validate_image(data: &[u8]) -> Result<&'static str, ImageError> {
    let (format, extension) = match image::guess_format(data) {
        Ok(ImageFormat::Png) => (ImageFormat::Png, "png"),
        Ok(ImageFormat::Jpeg) => (ImageFormat::Jpeg, "jpg"),
        _ => return Err(ImageError::Unsupported),
    };
    let reader = || ImageReader::with_format(Cursor::new(data), format);
    let (width, height) = reader()
        .into_dimensions()
        .map_err(|_| ImageError::Invalid)?;
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(ImageError::Invalid);
    }
    let mut reader = reader();
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(96 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().map_err(|_| ImageError::Invalid)?;
    Ok(extension)
}

#[cfg(test)]
pub(crate) fn test_png() -> Vec<u8> {
    let mut file = Cursor::new(Vec::new());
    image::RgbImage::from_pixel(2, 2, image::Rgb([90, 40, 150]))
        .write_to(&mut file, ImageFormat::Png)
        .unwrap();
    file.into_inner()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::storage::LocalFileStore;

    #[test]
    fn validates_contents_and_stores_by_hash() {
        let directory = tempfile::tempdir().unwrap();
        let store = ImageStore::new(LocalFileStore::new(directory.path().into()));
        let data = test_png();
        let name = store.save(&data).unwrap();
        assert_eq!(mime_type(&name), Some("image/png"));
        assert_eq!(store.save(&data).unwrap(), name);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        assert_eq!(store.read(&name, MAX_IMAGE_SIZE).unwrap(), data);
        assert!(matches!(
            store.save(b"<svg/>"),
            Err(ImageError::Unsupported)
        ));
        assert!(matches!(store.save(&data[..16]), Err(ImageError::Invalid)));
        assert!(store.read("../file.png", MAX_IMAGE_SIZE).is_err());
    }
}
