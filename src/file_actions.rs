use anyhow::Result;
use std::path::{Path, PathBuf};

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
