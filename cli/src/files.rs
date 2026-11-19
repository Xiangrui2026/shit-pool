use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use crate::error::{CliError, Result};

pub(crate) fn read_image(path: &Path) -> Result<Vec<u8>> {
    const MAX_BYTES: u64 = 8 * 1024 * 1024;
    if !fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
        return Err(CliError::io("Provide a regular image file."));
    }
    let mut data = Vec::new();
    File::open(path)
        .and_then(|file| file.take(MAX_BYTES + 1).read_to_end(&mut data))
        .map_err(|_| CliError::io("Could not read the image file."))?;
    if data.is_empty() || data.len() as u64 > MAX_BYTES {
        return Err(CliError::usage("Provide a non-empty image up to 8 MiB."));
    }
    Ok(data)
}

pub(crate) fn write_zip(path: &Path, data: &[u8]) -> Result<()> {
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| {
            CliError::io("Could not create the output file. Existing files are not overwritten.")
        })?;
    if file.write_all(data).is_err() {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(CliError::io("Could not write the ZIP file."));
    }
    Ok(())
}
