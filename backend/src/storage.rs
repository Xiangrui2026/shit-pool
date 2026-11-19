use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
};

#[derive(Clone, Copy)]
pub(crate) struct FileKey<'a>(&'a str);

impl<'a> FileKey<'a> {
    pub(crate) fn new(name: &'a str) -> Option<Self> {
        if name.len() > 255
            || !name
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            || name.ends_with('.')
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        {
            return None;
        }

        let base = name.split('.').next()?.to_ascii_uppercase();
        if matches!(base.as_str(), "AUX" | "CON" | "NUL" | "PRN")
            || matches!(
                base.as_bytes(),
                [b'C', b'O', b'M', b'1'..=b'9'] | [b'L', b'P', b'T', b'1'..=b'9']
            )
        {
            return None;
        }
        Some(Self(name))
    }

    pub(crate) fn as_str(self) -> &'a str {
        self.0
    }
}

pub(crate) trait FileStore: Send + Sync {
    fn put_if_absent(&self, key: FileKey<'_>, data: &[u8]) -> io::Result<()>;
    fn read(&self, key: FileKey<'_>, max_bytes: usize) -> io::Result<Vec<u8>>;
    fn remove(&self, key: FileKey<'_>) -> io::Result<()>;
}

pub(crate) struct LocalFileStore(PathBuf);

impl LocalFileStore {
    pub(crate) fn new(directory: PathBuf) -> Self {
        Self(directory)
    }
}

impl FileStore for LocalFileStore {
    fn put_if_absent(&self, key: FileKey<'_>, data: &[u8]) -> io::Result<()> {
        fs::create_dir_all(&self.0)?;
        let mut temporary = tempfile::NamedTempFile::new_in(&self.0)?;
        temporary.write_all(data)?;
        let path = self.0.join(key.as_str());
        match temporary.persist_noclobber(&path) {
            Ok(_) => Ok(()),
            Err(error)
                if error.error.kind() == io::ErrorKind::AlreadyExists
                    && fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_file()) =>
            {
                Ok(())
            }
            Err(error) => Err(error.error),
        }
    }

    fn read(&self, key: FileKey<'_>, max_bytes: usize) -> io::Result<Vec<u8>> {
        let path = self.0.join(key.as_str());
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file() {
            return Err(io::ErrorKind::NotFound.into());
        }
        if metadata.len() > max_bytes as u64 {
            return Err(io::ErrorKind::FileTooLarge.into());
        }
        let mut data = Vec::with_capacity(metadata.len() as usize);
        File::open(path)?
            .take((max_bytes as u64).saturating_add(1))
            .read_to_end(&mut data)?;
        if data.len() > max_bytes {
            return Err(io::ErrorKind::FileTooLarge.into());
        }
        Ok(data)
    }

    fn remove(&self, key: FileKey<'_>) -> io::Result<()> {
        match fs::remove_file(self.0.join(key.as_str())) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_portable_file_keys_only() {
        assert!(FileKey::new("question-42.zip").is_some());
        for name in [
            "",
            ".",
            "..",
            "../file",
            r"..\file",
            r"C:\file",
            "dir/file",
            "NUL.png",
            "com1",
            "suffix.",
            "file:stream",
        ] {
            assert!(FileKey::new(name).is_none(), "{name}");
        }
    }

    #[test]
    fn local_store_preserves_existing_data_and_caps_reads() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalFileStore::new(directory.path().join("objects"));
        let key = FileKey::new("asset.bin").unwrap();
        assert_eq!(
            store.read(key, 4).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        store.put_if_absent(key, b"test").unwrap();
        store.put_if_absent(key, b"overwritten").unwrap();
        assert_eq!(store.read(key, 4).unwrap(), b"test");
        assert_eq!(
            store.read(key, 3).unwrap_err().kind(),
            io::ErrorKind::FileTooLarge
        );
        store.remove(key).unwrap();
        store.remove(key).unwrap();
        assert_eq!(
            store.read(key, 4).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }
}
