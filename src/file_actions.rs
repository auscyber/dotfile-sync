use anyhow::{bail, Context, Result};
use std::{fs, path::Path, path::PathBuf};

pub fn copy_path(src: impl AsRef<Path>, destination: impl AsRef<Path>) -> Result<()> {
    let (src, destination) = (src.as_ref(), destination.as_ref().to_path_buf());

    if src.is_file() {
        fs::copy(src, destination)?;
    } else if src.is_dir() {
        copy_folder(src, &destination)?;
    } else {
        bail!("{} does not exist", src.display());
    }
    Ok(())
}

pub fn copy_folder<T: AsRef<std::path::Path>>(path: T, destination: T) -> Result<()> {
    let path = path.as_ref();
    let destination = destination.as_ref();
    if !path.exists() {
        bail!("Folder does not exist: {}", path.display());
    }
    if !path.is_dir() {
        bail!("Folder isn't actually folder: {}", path.display());
    }
    if destination.is_file() {
        bail!("Destination is file");
    }
    if !destination.exists() {
        fs::create_dir_all(destination)?;
    }
    for entry in fs::read_dir(path)? {
        let file = entry?.path();
        let subdest = destination.join(file.file_name().context("could not get file_name")?);
        if !file.exists() {
            bail!("File does not exist: {}", file.display());
        }
        if file.is_file() {
            fs::copy(file, subdest)?;
        } else {
            copy_folder(&file, &subdest).with_context(|| {
                format!(
                    "Failure copying folder {} to {}",
                    file.clone().display(),
                    subdest.clone().display()
                )
            })?;
        }
    }
    Ok(())
}
pub fn check_path(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        anyhow::bail!("File does not exist: {}", path.display());
    }
    if path
        .metadata()
        .map(|x| x.permissions().readonly())
        .unwrap_or(true)
    {
        anyhow::bail!("Invalid permissions for file: {}", path.display());
    }

    Ok(path.to_path_buf())
}
