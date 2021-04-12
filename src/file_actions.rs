use anyhow::{bail, Context, Result};
use std::{fs, path::PathBuf};

pub fn copy_path<T: AsRef<std::path::Path>, U: AsRef<std::path::Path>>(
    src: T,
    destination: U,
) -> Result<()> {
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

pub fn mv_link(path: &PathBuf, destination: &PathBuf) -> Result<()> {
    let mut destination = destination.clone();
    if destination.exists() && destination.is_file() {
        bail!("File already exists {}", destination.display());
    }
    if let Some(x) = destination.parent() {
        fs::create_dir_all(x)?;
    }
    if path.is_file() {
        if destination.is_dir() {
            destination.push(match path.file_name() {
                Some(x) => x,
                None => bail!("not a valid file_name"),
            });
        }
        if destination.is_file() {
            bail!("File already exists: {}", destination.display());
        }
        let full_path = path.canonicalize()?;
        fs::copy(path, &destination)?;
        fs::remove_file(path)?;
        fs::soft_link(destination, full_path)?;
    } else if path.is_dir() {
        if destination.is_file() {
            bail!("File already exists: {}", destination.display());
        }
        copy_folder(path, &destination)?;
        fs::remove_dir_all(path).context("Failure removing path")?;
        fs::soft_link(&destination.canonicalize()?, path).context("failure linking folder")?;
        drop((path, destination));
    } else {
        bail!("File is not file or directory")
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
pub fn check_path(path: &PathBuf) -> Result<PathBuf> {
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

    Ok(path.clone())
}
